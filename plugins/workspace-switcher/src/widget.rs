use crate::config::WorkspaceSwitcherConfig;
use crate::personalization::PersonalizationOverride;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::Button;
use gtk4::CssProvider;
use gtk4::Image;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
use smearor_model_compositor::CreateWorkspaceMessage;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_compositor::TOPIC_WORKSPACE_CHANGED;
use smearor_model_compositor::TOPIC_WORKSPACE_LIFECYCLE;
use smearor_model_compositor::TOPIC_WORKSPACE_SNAPSHOT;
use smearor_model_compositor::WorkspaceChangedEvent;
use smearor_model_compositor::WorkspaceCreatePosition;
use smearor_model_compositor::WorkspaceInfo;
use smearor_model_compositor::WorkspaceLifecycleEvent;
use smearor_model_compositor::WorkspaceLifecycleType;
use smearor_model_compositor::WorkspaceSnapshotMessage;
use smearor_model_compositor::WorkspaceSnapshotRequestMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TOPIC_STATUS as TOPIC_PERSONALIZATION_STATUS;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::GestureHandler;
use smearor_swipe_launcher_plugin_api::GestureHandlersConfiguration;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::apply_icon_color;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::debug;
use tracing::trace;

/// Workspace Switcher Widget.
///
/// A compact, touch-optimized widget that displays the current workspace
/// and allows switching between workspaces via swipe gestures. One workspace
/// per view, with dynamic creation when swiping past the edges.
pub struct WorkspaceSwitcherWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WorkspaceSwitcherConfig,
    /// Dynamic list of workspaces (one per view).
    pub workspaces: Rc<RefCell<Vec<WorkspaceInfo>>>,
    /// Index of the currently displayed workspace in the view list.
    pub current_view: Rc<RefCell<usize>>,
    /// The icon image widget.
    pub icon_image: Rc<RefCell<Option<Image>>>,
    /// The main label widget showing the workspace name.
    pub main_label: Rc<RefCell<Option<Label>>>,
    /// The info label widget showing the workspace index.
    pub info_label: Rc<RefCell<Option<Label>>>,
    /// The scrollbar indicator showing position in the workspace list.
    pub scrollbar: Rc<RefCell<Option<LevelBar>>>,
    /// Personalization override data (locale for sorting).
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl WorkspaceSwitcherWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: WorkspaceSwitcherConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget = WorkspaceSwitcherWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            workspaces: Rc::new(RefCell::new(Vec::new())),
            current_view: Rc::new(RefCell::new(0)),
            icon_image: Rc::new(RefCell::new(None)),
            main_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            scrollbar: Rc::new(RefCell::new(None)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };

        // Request initial workspace snapshot
        let broadcaster = widget.get_broadcaster();
        broadcaster.broadcast_message_to_topic(WorkspaceSnapshotRequestMessage { monitor_index: 0 });

        // Request personalization status for locale-aware sorting
        widget.request_personalization_status();

        Ok(widget)
    }

    fn next_view(&self) {
        let workspaces = self.workspaces.clone();
        let current_view = self.current_view.clone();
        let broadcaster = self.get_broadcaster();

        let icon_image = self.icon_image.clone();
        let main_label = self.main_label.clone();
        let info_label = self.info_label.clone();
        let scrollbar = self.scrollbar.clone();
        let config = self.config.clone();

        MainContext::default().spawn_local(async move {
            let (idx, has_next, next_id, last_id) = {
                let ws_list = workspaces.borrow();
                if ws_list.is_empty() {
                    return;
                }
                let idx = *current_view.borrow();
                let has_next = idx + 1 < ws_list.len();
                let next_id = if has_next { ws_list[idx + 1].workspace_id } else { -1 };
                let last_id = ws_list[idx].workspace_id;
                (idx, has_next, next_id, last_id)
            };

            if has_next {
                let msg = SwitchWorkspaceMessage { workspace_id: next_id };
                broadcaster.broadcast_message_to_topic(msg);
                *current_view.borrow_mut() = idx + 1;
                update_ui_internal(&workspaces, &current_view, &icon_image, &main_label, &info_label, &scrollbar, &config);
            } else {
                let msg = CreateWorkspaceMessage {
                    relative_to: last_id,
                    position: WorkspaceCreatePosition::After,
                };
                broadcaster.broadcast_message_to_topic(msg);
            }
        });
    }

    fn prev_view(&self) {
        let workspaces = self.workspaces.clone();
        let current_view = self.current_view.clone();
        let broadcaster = self.get_broadcaster();

        let icon_image = self.icon_image.clone();
        let main_label = self.main_label.clone();
        let info_label = self.info_label.clone();
        let scrollbar = self.scrollbar.clone();
        let config = self.config.clone();

        MainContext::default().spawn_local(async move {
            let (idx, has_prev, prev_id, first_id) = {
                let ws_list = workspaces.borrow();
                if ws_list.is_empty() {
                    return;
                }
                let idx = *current_view.borrow();
                let has_prev = idx > 0;
                let prev_id = if has_prev { ws_list[idx - 1].workspace_id } else { -1 };
                let first_id = ws_list[idx].workspace_id;
                (idx, has_prev, prev_id, first_id)
            };

            if has_prev {
                let msg = SwitchWorkspaceMessage { workspace_id: prev_id };
                broadcaster.broadcast_message_to_topic(msg);
                *current_view.borrow_mut() = idx - 1;
                update_ui_internal(&workspaces, &current_view, &icon_image, &main_label, &info_label, &scrollbar, &config);
            } else {
                let msg = CreateWorkspaceMessage {
                    relative_to: first_id,
                    position: WorkspaceCreatePosition::Before,
                };
                broadcaster.broadcast_message_to_topic(msg);
            }
        });
    }

    fn update_ui(&self) {
        let workspaces = self.workspaces.clone();
        let current_view = self.current_view.clone();
        let icon_image = self.icon_image.clone();
        let main_label = self.main_label.clone();
        let info_label = self.info_label.clone();
        let scrollbar = self.scrollbar.clone();
        let config = self.config.clone();

        MainContext::default().spawn_local(async move {
            update_ui_internal(&workspaces, &current_view, &icon_image, &main_label, &info_label, &scrollbar, &config);
        });
    }

    fn request_personalization_status(&self) {
        self.get_broadcaster()
            .broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }
}

impl DefaultFallback for WorkspaceSwitcherWidget {
    fn default_fallback(&self, kind: &ActionKind, _broadcaster: &MessageBroadcasterInner) {
        match kind {
            ActionKind::SwipeUp | ActionKind::ScrollUp | ActionKind::MiddleClick => {
                self.next_view();
            }
            ActionKind::SwipeDown | ActionKind::ScrollDown => {
                self.prev_view();
            }
            ActionKind::Click
            | ActionKind::DoublePress
            | ActionKind::Longpress
            | ActionKind::RightClick
            | ActionKind::Hold
            | ActionKind::CompoundLongpress
            | ActionKind::Init => {}
        }
    }
}

fn update_ui_internal(
    workspaces: &Rc<RefCell<Vec<WorkspaceInfo>>>,
    current_view: &Rc<RefCell<usize>>,
    icon_image: &Rc<RefCell<Option<Image>>>,
    main_label: &Rc<RefCell<Option<Label>>>,
    info_label: &Rc<RefCell<Option<Label>>>,
    scrollbar: &Rc<RefCell<Option<LevelBar>>>,
    config: &WorkspaceSwitcherConfig,
) {
    let ws_list = workspaces.borrow();
    let icon_size = config.icon_config.icon_size();
    let show_labels = !config.icon_config.icon_only();

    if ws_list.is_empty() {
        if let Some(ref image) = *icon_image.borrow() {
            set_workspace_icon(image, "nf-md-loading", icon_size);
        }
        if let Some(ref label) = *main_label.borrow() {
            label.set_text(if show_labels { "..." } else { "" });
        }
        if let Some(ref label) = *info_label.borrow() {
            label.set_text("");
        }
        if let Some(ref bar) = *scrollbar.borrow() {
            bar.set_value(0.0);
        }
        return;
    }

    let idx = *current_view.borrow();
    let idx = idx.min(ws_list.len() - 1);
    let ws = &ws_list[idx];

    if let Some(ref image) = *icon_image.borrow() {
        let icon_class = resolve_workspace_icon(config, ws.workspace_id);
        set_workspace_icon(image, &icon_class, icon_size);
    }
    if let Some(ref label) = *main_label.borrow() {
        if show_labels && config.show_label {
            label.set_text(&ws.workspace_name.to_string());
            label.set_visible(true);
        } else {
            label.set_visible(false);
        }
    }
    if let Some(ref label) = *info_label.borrow() {
        if show_labels {
            let info = format!("{}/{}", idx + 1, ws_list.len());
            label.set_text(&info);
            label.set_visible(true);
        } else {
            label.set_visible(false);
        }
    }
    if let Some(ref bar) = *scrollbar.borrow() {
        if config.show_scrollbar && ws_list.len() > 1 {
            let fraction = if ws_list.len() > 1 { idx as f64 / (ws_list.len() - 1) as f64 } else { 0.0 };
            bar.set_value(fraction);
            bar.set_visible(true);
        } else {
            bar.set_visible(false);
        }
    }
}

/// Sort workspaces by workspace_id, preserving a stable order.
/// Locale-aware collation could be applied here in the future for
/// sorting by workspace_name instead of by id.
fn sort_workspaces(ws_list: &mut Vec<WorkspaceInfo>, _personalization: &PersonalizationOverride) {
    ws_list.sort_by_key(|w| w.workspace_id);
}

fn resolve_workspace_icon(config: &WorkspaceSwitcherConfig, workspace_id: i32) -> String {
    let key = workspace_id.to_string();
    config.icon_map.get(&key).cloned().unwrap_or_else(|| config.default_icon.clone())
}

fn set_workspace_icon(image: &Image, icon_class: &str, icon_size: i32) {
    if icon_class.starts_with("nf-") {
        if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_class) {
            image.set_icon_name(Some(&gtk_icon_name));
        } else {
            image.set_icon_name(Some(icon_class));
        }
    } else {
        image.set_icon_name(Some(icon_class));
    }
    image.set_pixel_size(icon_size);
}

impl MessageHandler<WorkspaceSnapshotMessage> for WorkspaceSwitcherWidget {
    fn handle_message(&self, message: WorkspaceSnapshotMessage, _sender_id: &str) {
        debug!(
            "Workspace switcher: received snapshot with {} workspaces, active={}",
            message.workspaces.len(),
            message.active_workspace_id
        );
        let mut ws_list = self.workspaces.borrow_mut();
        ws_list.clear();
        for ws in message.workspaces.iter() {
            ws_list.push(ws.clone());
        }
        sort_workspaces(&mut ws_list, &self.personalization.borrow());

        let active_idx = ws_list.iter().position(|w| w.workspace_id == message.active_workspace_id);
        drop(ws_list);

        *self.current_view.borrow_mut() = active_idx.unwrap_or(0);
        self.update_ui();
    }
}

impl MessageHandler<WorkspaceChangedEvent> for WorkspaceSwitcherWidget {
    fn handle_message(&self, message: WorkspaceChangedEvent, _sender_id: &str) {
        debug!("Workspace switcher: workspace changed to {} (id={})", message.workspace_name, message.workspace_id);
        {
            let mut ws_list = self.workspaces.borrow_mut();
            for ws in ws_list.iter_mut() {
                ws.is_active = ws.workspace_id == message.workspace_id;
            }
            let active_idx = ws_list.iter().position(|w| w.workspace_id == message.workspace_id);
            drop(ws_list);

            if let Some(idx) = active_idx {
                *self.current_view.borrow_mut() = idx;
            }
        }
        self.update_ui();
    }
}

impl MessageHandler<WorkspaceLifecycleEvent> for WorkspaceSwitcherWidget {
    fn handle_message(&self, message: WorkspaceLifecycleEvent, _sender_id: &str) {
        debug!(
            "Workspace switcher: lifecycle event {:?} for workspace {} (id={})",
            message.lifecycle_type, message.workspace_name, message.workspace_id
        );
        {
            let mut ws_list = self.workspaces.borrow_mut();
            match message.lifecycle_type {
                WorkspaceLifecycleType::Created => {
                    let exists = ws_list.iter().any(|w| w.workspace_id == message.workspace_id);
                    if !exists {
                        ws_list.push(WorkspaceInfo {
                            workspace_id: message.workspace_id,
                            workspace_name: message.workspace_name.clone(),
                            monitor_index: message.monitor_index,
                            is_active: false,
                        });
                        sort_workspaces(&mut ws_list, &self.personalization.borrow());
                    }
                }
                WorkspaceLifecycleType::Destroyed => {
                    ws_list.retain(|w| w.workspace_id != message.workspace_id);
                }
            }
            drop(ws_list);
        }
        self.update_ui();
    }
}

impl AcceptTopic<FfiEnvelope> for WorkspaceSwitcherWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_WORKSPACE_SNAPSHOT
            || topic == TOPIC_WORKSPACE_CHANGED
            || topic == TOPIC_WORKSPACE_LIFECYCLE
            || topic == TOPIC_MCP_INVOKE_TOOL
            || topic == TOPIC_PERSONALIZATION_STATUS
    }
}

impl MessageBroadcaster for WorkspaceSwitcherWidget {}

impl PluginMetaGetter for WorkspaceSwitcherWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for WorkspaceSwitcherWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for WorkspaceSwitcherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let arguments = message.0.arguments.to_string();
        debug!("Workspace switcher: InvokeToolMessage name={} args={}", tool_name, arguments);

        let own_button_name = format!("button_{}", self.meta.id);
        if tool_name == own_button_name {
            let action_str = serde_json::from_str::<serde_json::Value>(&arguments)
                .ok()
                .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "click".to_string());

            match action_str.as_str() {
                "click" => self.next_view(),
                "longpress" => self.prev_view(),
                _ => debug!("Workspace switcher: unhandled action '{}'", action_str),
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for WorkspaceSwitcherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("workspace switcher: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride { locale };
        *self.personalization.borrow_mut() = override_data;
        // Re-sort workspaces with locale awareness and refresh UI
        {
            let mut ws_list = self.workspaces.borrow_mut();
            sort_workspaces(&mut ws_list, &self.personalization.borrow());
        }
        self.update_ui();
    }
}

impl WidgetPlugin for WorkspaceSwitcherWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            let topic = envelope.topic.to_string();
            if topic.starts_with("compositor.") {
                trace!("Workspace switcher: on_message topic={} type_id={}", topic, envelope.type_id);
            }
            if envelope.type_id == WorkspaceSnapshotMessage::TYPE_ID {
                MessageHandler::<WorkspaceSnapshotMessage>::handle_envelope_message(self, envelope);
            } else if envelope.type_id == WorkspaceChangedEvent::TYPE_ID {
                MessageHandler::<WorkspaceChangedEvent>::handle_envelope_message(self, envelope);
            } else if envelope.type_id == WorkspaceLifecycleEvent::TYPE_ID {
                MessageHandler::<WorkspaceLifecycleEvent>::handle_envelope_message(self, envelope);
            } else if envelope.type_id == <FfiEnvelopePayload<InvokeToolMessage> as TypedMessage>::TYPE_ID {
                MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
            } else if envelope.type_id == <FfiEnvelopePayload<PersonalizationStatusMessage> as TypedMessage>::TYPE_ID {
                MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
            }
        }
    }
}

impl WidgetBuilder for WorkspaceSwitcherWidget {
    fn build_widget(&mut self) -> Widget {
        let content_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(self.config.layout.spacing_or_default())
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["workspace-switcher-widget", "menu_button_inner"])
            .build();

        let icon_size = self.config.icon_config.icon_size();
        let show_labels = !self.config.icon_config.icon_only();

        let icon_image = Image::builder().css_classes(["workspace-switcher-icon", "nerd-icon"]).build();
        set_workspace_icon(&icon_image, &self.config.default_icon, icon_size);
        if let Some(color) = self.config.icon_config.icon_color() {
            apply_icon_color(&icon_image, color);
        }
        content_box.append(&icon_image);
        *self.icon_image.borrow_mut() = Some(icon_image.clone());

        match self.config.mode {
            WidgetMode::Compact => {
                let main_label = Label::builder()
                    .label(if show_labels { "..." } else { "" })
                    .css_classes(["widget-main-text"])
                    .build();
                main_label.set_height_request(20);
                apply_text_color(&main_label, self.config.text_colors.main_text_color());
                content_box.append(&main_label);
                *self.main_label.borrow_mut() = Some(main_label.clone());

                let info_label = Label::builder()
                    .label(if show_labels { "0/0" } else { "" })
                    .css_classes(["widget-info-text"])
                    .build();
                info_label.set_height_request(16);
                apply_text_color(&info_label, self.config.text_colors.info_text_color());
                content_box.append(&info_label);
                *self.info_label.borrow_mut() = Some(info_label.clone());

                let scrollbar = LevelBar::builder()
                    .min_value(0.0)
                    .max_value(1.0)
                    .value(0.0)
                    .width_request(self.config.dimensions.width_or_default() - 20)
                    .height_request(16)
                    .css_classes(["workspace-switcher-scrollbar"])
                    .build();
                content_box.append(&scrollbar);
                *self.scrollbar.borrow_mut() = Some(scrollbar.clone());
            }
            WidgetMode::Wide => {
                let info_box = GtkBox::builder()
                    .orientation(Orientation::Vertical)
                    .spacing(self.config.layout.spacing_or_default())
                    .valign(Align::Center)
                    .halign(Align::Start)
                    .build();

                let main_label = Label::builder()
                    .label(if show_labels { "..." } else { "" })
                    .ellipsize(gtk4::gdk::pango::EllipsizeMode::End)
                    .max_width_chars(24)
                    .css_classes(["widget-main-text"])
                    .build();
                main_label.set_height_request(20);
                apply_text_color(&main_label, self.config.text_colors.main_text_color());
                info_box.append(&main_label);
                *self.main_label.borrow_mut() = Some(main_label.clone());

                let info_label = Label::builder()
                    .label(if show_labels { "0/0" } else { "" })
                    .ellipsize(gtk4::gdk::pango::EllipsizeMode::End)
                    .max_width_chars(24)
                    .css_classes(["widget-info-text"])
                    .build();
                info_label.set_height_request(16);
                apply_text_color(&info_label, self.config.text_colors.info_text_color());
                info_box.append(&info_label);
                *self.info_label.borrow_mut() = Some(info_label.clone());

                let scrollbar = LevelBar::builder()
                    .min_value(0.0)
                    .max_value(1.0)
                    .value(0.0)
                    .width_request(self.config.dimensions.max_width_or_default(self.config.mode) - 20)
                    .height_request(16)
                    .css_classes(["workspace-switcher-scrollbar"])
                    .build();
                info_box.append(&scrollbar);
                *self.scrollbar.borrow_mut() = Some(scrollbar.clone());

                content_box.append(&info_box);
            }
        }

        let effective_width = self
            .config
            .dimensions
            .width_or_default()
            .min(self.config.dimensions.max_width_or_default(self.config.mode));
        let mut button_builder = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(effective_width)
            .height_request(self.config.dimensions.height_or_default())
            .child(&content_box);
        if let Some(max_w) = self.config.dimensions.max_width {
            button_builder = button_builder.hexpand(false).halign(Align::Start);
            let css_class = format!("max-width-{}", max_w);
            button_builder = button_builder.css_classes(["scroll-item", "menu-button", css_class.as_str()]);
            let css = format!(".max-width-{} {{ max-width: {}px; }}", max_w, max_w);
            if let Some(display) = gtk4::gdk::Display::default() {
                let provider = CssProvider::new();
                provider.load_from_string(&css);
                gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
            }
        }
        let button = button_builder.build();

        let message_broadcaster = self.get_broadcaster();

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            workspaces: self.workspaces.clone(),
            current_view: self.current_view.clone(),
            icon_image: self.icon_image.clone(),
            main_label: self.main_label.clone(),
            info_label: self.info_label.clone(),
            scrollbar: self.scrollbar.clone(),
            personalization: self.personalization.clone(),
        });

        let button_widget = button.upcast::<Widget>();
        apply_widget_css_classes(&button_widget, &self.meta.id, &self.config.layout.css_classes);
        widget_self.attach_gesture_handlers(&button_widget, &self.config.actions, &message_broadcaster, &GestureHandlersConfiguration::default());

        button_widget
    }
}
