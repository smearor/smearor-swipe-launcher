use crate::config::WorkspaceSwitcherConfig;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureDrag;
use gtk4::GestureLongPress;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
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
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::Plugin;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::debug;

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
    /// The label widget showing the workspace name.
    pub label_widget: Rc<RefCell<Option<Label>>>,
    /// The dot indicator container.
    pub dot_container: Rc<RefCell<Option<GtkBox>>>,
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
            label_widget: Rc::new(RefCell::new(None)),
            dot_container: Rc::new(RefCell::new(None)),
        };

        // Request initial workspace snapshot
        let broadcaster = widget.get_broadcaster();
        broadcaster.broadcast_message_to_topic(WorkspaceSnapshotRequestMessage { monitor_index: 0 });

        Ok(widget)
    }

    fn next_view(&self) {
        let workspaces = self.workspaces.clone();
        let current_view = self.current_view.clone();
        let broadcaster = self.get_broadcaster();

        let icon_image = self.icon_image.clone();
        let label_widget = self.label_widget.clone();
        let dot_container = self.dot_container.clone();
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
                update_ui_internal(&workspaces, &current_view, &icon_image, &label_widget, &dot_container, &config);
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
        let label_widget = self.label_widget.clone();
        let dot_container = self.dot_container.clone();
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
                update_ui_internal(&workspaces, &current_view, &icon_image, &label_widget, &dot_container, &config);
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
        let label_widget = self.label_widget.clone();
        let dot_container = self.dot_container.clone();
        let config = self.config.clone();

        MainContext::default().spawn_local(async move {
            update_ui_internal(&workspaces, &current_view, &icon_image, &label_widget, &dot_container, &config);
        });
    }
}

fn update_ui_internal(
    workspaces: &Rc<RefCell<Vec<WorkspaceInfo>>>,
    current_view: &Rc<RefCell<usize>>,
    icon_image: &Rc<RefCell<Option<Image>>>,
    label_widget: &Rc<RefCell<Option<Label>>>,
    dot_container: &Rc<RefCell<Option<GtkBox>>>,
    config: &WorkspaceSwitcherConfig,
) {
    let ws_list = workspaces.borrow();

    if ws_list.is_empty() {
        if let Some(ref image) = *icon_image.borrow() {
            set_workspace_icon(image, "nf-md-loading", config.icon_size);
        }
        if let Some(ref label) = *label_widget.borrow() {
            label.set_text("...");
        }
        if let Some(ref container) = *dot_container.borrow() {
            update_dot_indicator(container, 0, 0);
        }
        return;
    }

    let idx = *current_view.borrow();
    let idx = idx.min(ws_list.len() - 1);
    let ws = &ws_list[idx];

    if let Some(ref image) = *icon_image.borrow() {
        let icon_class = resolve_workspace_icon(config, ws.workspace_id);
        set_workspace_icon(image, &icon_class, config.icon_size);
    }
    if let Some(ref label) = *label_widget.borrow() {
        if config.show_label {
            label.set_text(&ws.workspace_name.to_string());
            label.set_visible(true);
        } else {
            label.set_visible(false);
        }
    }
    if let Some(ref container) = *dot_container.borrow() {
        if config.show_dot_indicator {
            update_dot_indicator(container, ws_list.len(), idx);
            container.set_visible(true);
        } else {
            container.set_visible(false);
        }
    }
}

fn resolve_workspace_icon(config: &WorkspaceSwitcherConfig, workspace_id: i32) -> String {
    let key = workspace_id.to_string();
    config.icon_map.get(&key).cloned().unwrap_or_else(|| config.default_icon.clone())
}

fn set_workspace_icon(image: &Image, icon_class: &str, icon_size: i32) {
    if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_class) {
        image.set_icon_name(Some(&gtk_icon_name));
    }
    image.set_pixel_size(icon_size);
}

fn update_dot_indicator(container: &GtkBox, total: usize, current: usize) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    for i in 0..total {
        let dot = Label::builder()
            .css_classes(if i == current {
                vec!["workspace-dot-active".to_string()]
            } else {
                vec!["workspace-dot-inactive".to_string()]
            })
            .build();
        dot.set_text("\u{2022}");
        container.append(&dot);
    }
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
        ws_list.sort_by_key(|w| w.workspace_id);

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
    }
}

impl AcceptTopic<FfiEnvelope> for WorkspaceSwitcherWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_WORKSPACE_SNAPSHOT || topic == TOPIC_WORKSPACE_CHANGED || topic == TOPIC_WORKSPACE_LIFECYCLE
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

impl Plugin for WorkspaceSwitcherWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            let topic = envelope.topic.to_string();
            if topic.starts_with("compositor.") {
                debug!("Workspace switcher: on_message topic={} type_id={}", topic, envelope.type_id);
            }
            if envelope.type_id == WorkspaceSnapshotMessage::TYPE_ID {
                MessageHandler::<WorkspaceSnapshotMessage>::handle_envelope_message(self, envelope);
            } else if envelope.type_id == WorkspaceChangedEvent::TYPE_ID {
                MessageHandler::<WorkspaceChangedEvent>::handle_envelope_message(self, envelope);
            } else if envelope.type_id == WorkspaceLifecycleEvent::TYPE_ID {
                MessageHandler::<WorkspaceLifecycleEvent>::handle_envelope_message(self, envelope);
            }
        }
    }
}

impl WidgetBuilder for WorkspaceSwitcherWidget {
    fn build_widget(&mut self) -> Widget {
        let outer_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .css_classes(["workspace-switcher-widget".to_string()])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        if let Some(width) = self.config.width {
            outer_box.set_width_request(width);
        }
        if let Some(height) = self.config.height {
            outer_box.set_height_request(height);
        }

        let icon_image = Image::builder().css_classes(["workspace-switcher-icon".to_string()]).build();
        let label_widget = Label::builder().css_classes(["workspace-switcher-label".to_string()]).build();
        let dot_container = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(2)
            .css_classes(["workspace-switcher-dots".to_string()])
            .halign(Align::Center)
            .build();

        set_workspace_icon(&icon_image, "nf-md-loading", self.config.icon_size);
        label_widget.set_text("...");
        update_dot_indicator(&dot_container, 0, 0);

        outer_box.append(&icon_image);
        outer_box.append(&label_widget);
        outer_box.append(&dot_container);

        *self.icon_image.borrow_mut() = Some(icon_image.clone());
        *self.label_widget.borrow_mut() = Some(label_widget.clone());
        *self.dot_container.borrow_mut() = Some(dot_container.clone());

        let click_topic = self.config.click_topic.clone();
        let click_payload = self.config.click_payload.clone();
        let longpress_topic = self.config.longpress_topic.clone();
        let longpress_payload = self.config.longpress_payload.clone();
        let message_broadcaster = self.get_broadcaster();

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            workspaces: self.workspaces.clone(),
            current_view: self.current_view.clone(),
            icon_image: self.icon_image.clone(),
            label_widget: self.label_widget.clone(),
            dot_container: self.dot_container.clone(),
        });

        let click_gesture = GestureClick::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_for_click = message_broadcaster.clone();
        click_gesture.connect_released(move |gesture, _n_press, _x, _y| {
            if let Some(seq) = gesture.current_sequence() {
                let state = gesture.sequence_state(&seq);
                if state == EventSequenceState::Claimed || state == EventSequenceState::Denied {
                    return;
                }
            }
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                let payload_str = payload.to_string();
                broadcaster_for_click.broadcast_string(&topic, &payload_str);
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        outer_box.add_controller(click_gesture);

        let drag_gesture = GestureDrag::new();
        drag_gesture.set_propagation_phase(PropagationPhase::Capture);
        let widget_for_drag = widget_self.clone();
        drag_gesture.connect_drag_end(move |gesture, offset_x, offset_y| {
            const SWIPE_THRESHOLD: f64 = 50.0;
            if offset_y.abs() > offset_x.abs() && offset_y.abs() > SWIPE_THRESHOLD {
                gesture.set_state(EventSequenceState::Claimed);
                if offset_y < 0.0 {
                    widget_for_drag.next_view();
                } else {
                    widget_for_drag.prev_view();
                }
            }
        });
        outer_box.add_controller(drag_gesture);

        let longpress_gesture = GestureLongPress::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_for_longpress = message_broadcaster.clone();
        longpress_gesture.connect_pressed(move |gesture, _x, _y| {
            if let (Some(topic), Some(payload)) = (longpress_topic.clone(), longpress_payload.clone()) {
                let payload_str = payload.to_string();
                broadcaster_for_longpress.broadcast_string(&topic, &payload_str);
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        outer_box.add_controller(longpress_gesture);

        outer_box.upcast::<Widget>()
    }
}
