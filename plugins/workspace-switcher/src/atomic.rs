use crate::config::WorkspaceAtomicConfig;
use crate::nav_target::WorkspaceNavTarget;
use gtk4::Label;
use gtk4::glib::MainContext;
use schemars::schema_for;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_compositor::TOPIC_WORKSPACE_CHANGED;
use smearor_model_compositor::TOPIC_WORKSPACE_LIFECYCLE;
use smearor_model_compositor::TOPIC_WORKSPACE_SNAPSHOT;
use smearor_model_compositor::WorkspaceChangedEvent;
use smearor_model_compositor::WorkspaceInfo;
use smearor_model_compositor::WorkspaceLifecycleEvent;
use smearor_model_compositor::WorkspaceLifecycleType;
use smearor_model_compositor::WorkspaceSnapshotMessage;
use smearor_model_compositor::WorkspaceSnapshotRequestMessage;
use smearor_model_mcp::ButtonActionArgs;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::AtomicAction;
use smearor_swipe_launcher_plugin_api::AtomicGraphicData;
use smearor_swipe_launcher_plugin_api::AtomicGraphicRenderer;
use smearor_swipe_launcher_plugin_api::AtomicWidgetBuildParams;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::build_atomic_widget;
use smearor_swipe_launcher_plugin_api::render_atomic_graphic_default;
use smearor_swipe_launcher_plugin_api::update_labels;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::debug;
use tracing::trace;

/// Which workspace view an atomic widget renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceAtomicView {
    /// Next workspace button — click switches to next workspace.
    Next,
    /// Previous workspace button — click switches to previous workspace.
    Previous,
    /// Current workspace name display.
    Name,
    /// Select a specific workspace by index.
    Select,
}

impl FromStr for WorkspaceAtomicView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "workspace_next" => Ok(Self::Next),
            "workspace_previous" => Ok(Self::Previous),
            "workspace_name" => Ok(Self::Name),
            "workspace_select" => Ok(Self::Select),
            _ => Err(format!("Unknown workspace atomic view: {s}")),
        }
    }
}

impl WorkspaceAtomicView {
    /// Returns the default nerd font icon name for this view.
    pub fn default_icon_name(&self) -> &'static str {
        match self {
            Self::Next => "nf-md-chevron_right",
            Self::Previous => "nf-md-chevron_left",
            Self::Name => "nf-md-monitor",
            Self::Select => "nf-md-monitor",
        }
    }

    /// Returns the default icon codepoint for this view.
    pub fn default_icon_char(&self) -> char {
        resolve_icon_codepoint(self.default_icon_name()).unwrap_or('\u{f1d8}')
    }
}

/// Atomic workspace widget that renders a single workspace view.
///
/// Subscribes to compositor events (`workspace_snapshot`, `workspace_changed`,
/// `workspace_lifecycle`) to track workspace state. Depending on the view:
/// - `Next` / `Previous`: Click switches to next/previous workspace,
///   longpress creates a new workspace.
/// - `Name`: Displays current workspace name and icon.
/// - `Select`: Click switches to the workspace at the configured `workspace_index`.
pub struct WorkspaceAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WorkspaceAtomicConfig,
    pub view: WorkspaceAtomicView,
    /// Dynamic list of workspaces (sorted by workspace_id).
    pub workspaces: Rc<RefCell<Vec<WorkspaceInfo>>>,
    /// Index of the currently active workspace.
    pub current_index: Rc<RefCell<usize>>,
    /// The icon label widget.
    pub icon_label: Rc<RefCell<Option<Label>>>,
    /// The main label widget showing the workspace name.
    pub main_label: Rc<RefCell<Option<Label>>>,
    /// The info label widget showing additional info.
    pub info_label: Rc<RefCell<Option<Label>>>,
}

impl WorkspaceAtomicWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: WorkspaceAtomicConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget_name = config.config.get("widget").and_then(|v| v.as_str()).unwrap_or_default();

        let view = WorkspaceAtomicView::from_str(widget_name).unwrap_or(WorkspaceAtomicView::Name);

        let widget = WorkspaceAtomicWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            view,
            workspaces: Rc::new(RefCell::new(Vec::new())),
            current_index: Rc::new(RefCell::new(0)),
            icon_label: Rc::new(RefCell::new(None)),
            main_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
        };

        widget.register_mcp_capabilities();
        widget.request_initial_snapshot();
        Ok(widget)
    }

    fn request_initial_snapshot(&self) {
        let broadcaster = MessageBroadcaster::get_broadcaster(self);
        broadcaster.broadcast_message_to_topic(WorkspaceSnapshotRequestMessage { monitor_index: 0 });
    }

    /// Resolve the icon for a given workspace ID.
    fn resolve_workspace_icon(&self, workspace_id: i32) -> char {
        if let Some(ref icon) = self.config.icon {
            return resolve_icon_codepoint(icon).unwrap_or(self.view.default_icon_char());
        }
        let key = workspace_id.to_string();
        let icon_name = self
            .config
            .icon_map
            .get(&key)
            .or_else(|| self.config.default_icon.as_ref())
            .map(|s| s.as_str())
            .unwrap_or_else(|| self.view.default_icon_name());
        resolve_icon_codepoint(icon_name).unwrap_or(self.view.default_icon_char())
    }

    /// Switch to the next workspace.
    fn next_workspace(&self) {
        let workspaces = self.workspaces.clone();
        let current_index = self.current_index.clone();
        let broadcaster = MessageBroadcaster::get_broadcaster(self);

        MainContext::default().spawn_local(async move {
            let Some(nav) = WorkspaceNavTarget::next(&workspaces.borrow(), *current_index.borrow()) else {
                return;
            };

            nav.broadcast_switch_or_create(&broadcaster);
        });
    }

    /// Switch to the previous workspace.
    fn prev_workspace(&self) {
        let workspaces = self.workspaces.clone();
        let current_index = self.current_index.clone();
        let broadcaster = MessageBroadcaster::get_broadcaster(self);

        MainContext::default().spawn_local(async move {
            let Some(nav) = WorkspaceNavTarget::prev(&workspaces.borrow(), *current_index.borrow()) else {
                return;
            };

            nav.broadcast_switch_or_create(&broadcaster);
        });
    }

    /// Switch to the workspace at the configured index.
    fn select_workspace(&self) {
        let workspaces = self.workspaces.clone();
        let broadcaster = MessageBroadcaster::get_broadcaster(self);
        let target_index = self.config.workspace_index.unwrap_or(0);

        MainContext::default().spawn_local(async move {
            let ws_list = workspaces.borrow();
            if ws_list.is_empty() {
                return;
            }
            let idx = target_index.min(ws_list.len() - 1);
            let workspace_id = ws_list[idx].workspace_id;
            let msg = SwitchWorkspaceMessage { workspace_id };
            broadcaster.broadcast_message_to_topic(msg);
        });
    }

    /// Update GTK labels from current workspace state.
    fn update_ui(&self) {
        let (icon_char, main_text, info_text) = self.render_view_data();
        update_labels(
            &*self.icon_label.borrow(),
            &*self.main_label.borrow(),
            &*self.info_label.borrow(),
            &icon_char.to_string(),
            &main_text,
            &info_text,
        );
        if let Some(ref label) = *self.main_label.borrow() {
            apply_text_color(label, self.config.atomic.text_colors.main_text_color());
        }
        if let Some(ref label) = *self.info_label.borrow() {
            apply_text_color(label, self.config.atomic.text_colors.info_text_color());
        }
    }

    /// Render view data from current workspace state.
    fn render_view_data(&self) -> (char, String, String) {
        let ws_list = self.workspaces.borrow();
        let show_label = self.config.show_label.unwrap_or(true);

        if ws_list.is_empty() {
            return (self.view.default_icon_char(), "...".to_string(), "".to_string());
        }

        match self.view {
            WorkspaceAtomicView::Next | WorkspaceAtomicView::Previous => {
                let icon = self
                    .config
                    .icon
                    .as_deref()
                    .map(|s| resolve_icon_codepoint(s).unwrap_or(self.view.default_icon_char()))
                    .unwrap_or(self.view.default_icon_char());
                (icon, "".to_string(), "".to_string())
            }
            WorkspaceAtomicView::Name => {
                let idx = *self.current_index.borrow();
                let idx = idx.min(ws_list.len() - 1);
                let ws = &ws_list[idx];
                let icon = self.resolve_workspace_icon(ws.workspace_id);
                let main = if show_label { ws.workspace_name.to_string() } else { "".to_string() };
                let info = format!("{}/{}", idx + 1, ws_list.len());
                (icon, main, info)
            }
            WorkspaceAtomicView::Select => {
                let target_idx = self.config.workspace_index.unwrap_or(0);
                let idx = target_idx.min(ws_list.len() - 1);
                let ws = &ws_list[idx];
                let icon = self.resolve_workspace_icon(ws.workspace_id);
                let main = if show_label { ws.workspace_name.to_string() } else { "".to_string() };
                (icon, main, "".to_string())
            }
        }
    }

    /// Extract graphic rendering data for the centralised rendering pipeline.
    fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
        let (icon_char, main_text, info_text) = self.render_view_data();
        let mut data = AtomicGraphicData::new(icon_char, main_text, info_text);
        data.main_text_color = self.config.atomic.text_colors.main_text_color().map(|c| c.to_rgba());
        data.info_text_color = self.config.atomic.text_colors.info_text_color().map(|c| c.to_rgba());
        data
    }

    /// Broadcast a widget update message.
    fn broadcast_widget_update(&self) {
        let plugin_id = self.meta.id.to_string();
        let msg = WidgetUpdateMessage::new(&plugin_id, "");
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(msg);
    }
}

impl McpCapabilitiesRegistrator for WorkspaceAtomicWidget {
    fn register_mcp_capabilities(&self) {
        if self.config.atomic.description.is_some() {
            let schema = serde_json::to_string(&schema_for!(ButtonActionArgs)).unwrap_or_default();
            let tool = RegisterToolMessage::new(
                &format!("button_{}", self.meta.id),
                self.config.atomic.description.as_deref().unwrap_or("Workspace atomic widget"),
                &schema,
            );
            MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(tool);
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for WorkspaceAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let arguments = message.0.arguments.to_string();
        debug!("Workspace atomic ({:?}): InvokeToolMessage name={} args={}", self.view, tool_name, arguments);

        let own_button_name = format!("button_{}", self.meta.id);
        if tool_name == own_button_name {
            let action_str = serde_json::from_str::<serde_json::Value>(&arguments)
                .ok()
                .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "click".to_string());
            let action = AtomicAction::from_str(&action_str).unwrap_or(AtomicAction::Click);

            let handled = match (self.view, action) {
                (WorkspaceAtomicView::Next, AtomicAction::Click) => {
                    self.next_workspace();
                    true
                }
                (WorkspaceAtomicView::Next, AtomicAction::Longpress) => {
                    self.next_workspace();
                    true
                }
                (WorkspaceAtomicView::Previous, AtomicAction::Click) => {
                    self.prev_workspace();
                    true
                }
                (WorkspaceAtomicView::Previous, AtomicAction::Longpress) => {
                    self.prev_workspace();
                    true
                }
                (WorkspaceAtomicView::Select, AtomicAction::Click) => {
                    self.select_workspace();
                    true
                }
                _ => {
                    self.config.atomic.dispatch_action(&MessageBroadcaster::get_broadcaster(self), action);
                    false
                }
            };

            let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("{} handled", action.as_ref()));
            MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(response);
            let _ = handled;
        }
    }
}

impl AcceptTopic<FfiEnvelope> for WorkspaceAtomicWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_WORKSPACE_SNAPSHOT || topic == TOPIC_WORKSPACE_CHANGED || topic == TOPIC_WORKSPACE_LIFECYCLE || topic == TOPIC_MCP_INVOKE_TOOL
    }
}

impl MessageBroadcaster for WorkspaceAtomicWidget {}

impl PluginMetaGetter for WorkspaceAtomicWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for WorkspaceAtomicWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl GraphicRenderer for WorkspaceAtomicWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        debug!("WorkspaceAtomicWidget ({:?}): render_graphic {}x{}", self.view, width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let data = self.render_atomic_graphic_data();

        render_atomic_graphic_default(
            &mut pixels,
            width,
            height,
            data.icon_char,
            &data.main_text,
            &data.info_text,
            data.is_error,
            &self.config.atomic,
            Some(self as &dyn AtomicGraphicRenderer),
            data.icon_color,
            data.main_text_color,
            data.info_text_color,
        );

        FfiGraphic::from_pixels(width, height, pixels)
    }
}

impl AtomicGraphicRenderer for WorkspaceAtomicWidget {
    fn render_graphic(&self, _pixels: &mut [u8], _width: u32, _height: u32) -> bool {
        false
    }
}

impl WidgetBuilder for WorkspaceAtomicWidget {
    fn build_widget(&mut self) -> gtk4::Widget {
        let params = AtomicWidgetBuildParams {
            css_prefix: "workspace",
            default_icon: '\u{f1d8}',
            default_main: "--",
            default_info: "Loading...",
        };
        let (widget, icon_label, main_label, info_label) = build_atomic_widget(&MessageBroadcaster::get_broadcaster(self), &self.config.atomic, &params);

        *self.icon_label.borrow_mut() = Some(icon_label);
        *self.main_label.borrow_mut() = Some(main_label);
        *self.info_label.borrow_mut() = Some(info_label);

        self.update_ui();

        apply_widget_css_classes(&widget, &self.meta.id, &[]);
        widget
    }
}

impl WidgetPlugin for WorkspaceAtomicWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            let topic = envelope.topic.to_string();
            trace!("Workspace atomic ({:?}): on_message topic={} type_id={}", self.view, topic, envelope.type_id);
            if envelope.type_id == WorkspaceSnapshotMessage::TYPE_ID {
                MessageHandler::<WorkspaceSnapshotMessage>::handle_envelope_message(self, envelope);
            } else if envelope.type_id == WorkspaceChangedEvent::TYPE_ID {
                MessageHandler::<WorkspaceChangedEvent>::handle_envelope_message(self, envelope);
            } else if envelope.type_id == WorkspaceLifecycleEvent::TYPE_ID {
                MessageHandler::<WorkspaceLifecycleEvent>::handle_envelope_message(self, envelope);
            } else if envelope.type_id == <FfiEnvelopePayload<InvokeToolMessage> as TypedMessage>::TYPE_ID {
                MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
            }
        }
    }
}

impl MessageHandler<WorkspaceSnapshotMessage> for WorkspaceAtomicWidget {
    fn handle_message(&self, message: WorkspaceSnapshotMessage, _sender_id: &str) {
        debug!(
            "Workspace atomic ({:?}): received snapshot with {} workspaces, active={}",
            self.view,
            message.workspaces.len(),
            message.active_workspace_id
        );
        let mut ws_list = self.workspaces.borrow_mut();
        ws_list.clear();
        for ws in message.workspaces.iter() {
            ws_list.push(ws.clone());
        }
        ws_list.sort_by_key(|w| w.workspace_id);

        let active_idx = ws_list.iter().position(|w| w.workspace_id == message.active_workspace_id);
        drop(ws_list);

        *self.current_index.borrow_mut() = active_idx.unwrap_or(0);
        self.update_ui();
        self.broadcast_widget_update();
    }
}

impl MessageHandler<WorkspaceChangedEvent> for WorkspaceAtomicWidget {
    fn handle_message(&self, message: WorkspaceChangedEvent, _sender_id: &str) {
        debug!(
            "Workspace atomic ({:?}): workspace changed to {} (id={})",
            self.view, message.workspace_name, message.workspace_id
        );
        {
            let mut ws_list = self.workspaces.borrow_mut();
            for ws in ws_list.iter_mut() {
                ws.is_active = ws.workspace_id == message.workspace_id;
            }
            let active_idx = ws_list.iter().position(|w| w.workspace_id == message.workspace_id);
            drop(ws_list);

            if let Some(idx) = active_idx {
                *self.current_index.borrow_mut() = idx;
            }
        }
        self.update_ui();
        self.broadcast_widget_update();
    }
}

impl MessageHandler<WorkspaceLifecycleEvent> for WorkspaceAtomicWidget {
    fn handle_message(&self, message: WorkspaceLifecycleEvent, _sender_id: &str) {
        debug!(
            "Workspace atomic ({:?}): lifecycle event {:?} for workspace {} (id={})",
            self.view, message.lifecycle_type, message.workspace_name, message.workspace_id
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
                        ws_list.sort_by_key(|w| w.workspace_id);
                    }
                }
                WorkspaceLifecycleType::Destroyed => {
                    ws_list.retain(|w| w.workspace_id != message.workspace_id);
                }
            }
            drop(ws_list);
        }
        self.update_ui();
        self.broadcast_widget_update();
    }
}
