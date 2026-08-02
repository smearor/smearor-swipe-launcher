pub mod rate_limiter;

pub use rate_limiter::RATE_LIMIT_MS;
pub use rate_limiter::RateLimiter;
pub use rate_limiter::StatusEvent;

use smearor_hyprland_model::ActiveWindowChangedStatusMessage;
use smearor_hyprland_model::ChangedSpecialStatusMessage;
use smearor_hyprland_model::ConfigReloadedStatusMessage;
use smearor_hyprland_model::FloatStateChangedStatusMessage;
use smearor_hyprland_model::FullscreenStateChangedStatusMessage;
use smearor_hyprland_model::GroupEvent;
use smearor_hyprland_model::GroupToggledStatusMessage;
use smearor_hyprland_model::HyprlandChangedSpecialEventData;
use smearor_hyprland_model::HyprlandGroupToggledEventData;
use smearor_hyprland_model::HyprlandLayoutEvent;
use smearor_hyprland_model::HyprlandNonSpecialWorkspaceData;
use smearor_hyprland_model::HyprlandScreencastEventData;
use smearor_hyprland_model::HyprlandScreencastType;
use smearor_hyprland_model::HyprlandWindowEventData;
use smearor_hyprland_model::HyprlandWindowFloatEventData;
use smearor_hyprland_model::HyprlandWindowMoveEvent;
use smearor_hyprland_model::HyprlandWindowOpenEvent;
use smearor_hyprland_model::HyprlandWindowPinEventData;
use smearor_hyprland_model::HyprlandWindowTitleEventData;
use smearor_hyprland_model::IgnoreGroupLockStateChangedStatusMessage;
use smearor_hyprland_model::KeyboardLayoutChangedStatusMessage;
use smearor_hyprland_model::LayerClosedStatusMessage;
use smearor_hyprland_model::LayerEvent;
use smearor_hyprland_model::LayerOpenedStatusMessage;
use smearor_hyprland_model::LockGroupsStateChangedStatusMessage;
use smearor_hyprland_model::ScreencastStatusMessage;
use smearor_hyprland_model::SpecialRemovedStatusMessage;
use smearor_hyprland_model::SubMapChangedStatusMessage;
use smearor_hyprland_model::SystemEvent;
use smearor_hyprland_model::UrgentStateChangedStatusMessage;
use smearor_hyprland_model::WindowClosedStatusMessage;
use smearor_hyprland_model::WindowEvent;
use smearor_hyprland_model::WindowMovedIntoGroupStatusMessage;
use smearor_hyprland_model::WindowMovedOutOfGroupStatusMessage;
use smearor_hyprland_model::WindowMovedStatusMessage;
use smearor_hyprland_model::WindowOpenedStatusMessage;
use smearor_hyprland_model::WindowPinnedStatusMessage;
use smearor_hyprland_model::WindowTitleChangedStatusMessage;
use smearor_hyprland_model::WorkspaceEvent;
use smearor_hyprland_model::WorkspaceRenamedStatusMessage;
use tokio::sync::mpsc;
use tracing::debug;

/// Convert a `hyprland::event_listener::WindowEventData` option to our model type.
fn convert_window_event_data(data: Option<hyprland::event_listener::WindowEventData>) -> stabby::option::Option<HyprlandWindowEventData> {
    data.map(|d| HyprlandWindowEventData {
        window_class: d.class.into(),
        window_title: d.title.into(),
        window_address: d.address.to_string().into(),
        workspace_id: 0,
    })
    .into()
}

/// Register all Hyprland-specific status handlers on the shared listener.
pub fn register_handlers(listener: &mut hyprland::event_listener::EventListener, sender: mpsc::UnboundedSender<StatusEvent>) {
    let s = sender.clone();
    listener.add_active_window_changed_handler(move |data| {
        let _ = s.send(StatusEvent::Window(WindowEvent::ActiveChanged(ActiveWindowChangedStatusMessage {
            data: convert_window_event_data(data),
        })));
    });

    let s = sender.clone();
    listener.add_fullscreen_state_changed_handler(move |is_fullscreen| {
        let _ = s.send(StatusEvent::Workspace(WorkspaceEvent::FullscreenStateChanged(FullscreenStateChangedStatusMessage {
            is_fullscreen,
        })));
    });

    let s = sender.clone();
    listener.add_window_opened_handler(move |data| {
        let event = HyprlandWindowOpenEvent {
            data: HyprlandWindowEventData {
                window_class: data.window_class.into(),
                window_title: data.window_title.into(),
                window_address: data.window_address.to_string().into(),
                workspace_id: 0,
            },
            floats: false,
            workspace_name: data.workspace_name.into(),
        };
        let _ = s.send(StatusEvent::Window(WindowEvent::Opened(WindowOpenedStatusMessage { data: event })));
    });

    let s = sender.clone();
    listener.add_window_closed_handler(move |data| {
        let _ = s.send(StatusEvent::Window(WindowEvent::Closed(WindowClosedStatusMessage {
            window_address: data.to_string().into(),
        })));
    });

    let s = sender.clone();
    listener.add_window_moved_handler(move |data| {
        let event = HyprlandWindowMoveEvent {
            window_address: data.window_address.to_string().into(),
            workspace_id: data.workspace_id,
        };
        let _ = s.send(StatusEvent::Window(WindowEvent::Moved(WindowMovedStatusMessage { data: event })));
    });

    let s = sender.clone();
    listener.add_layout_changed_handler(move |data| {
        let event = HyprlandLayoutEvent {
            keyboard_name: data.keyboard_name.into(),
            layout_name: data.layout_name.into(),
        };
        let _ = s.send(StatusEvent::System(SystemEvent::KeyboardLayoutChanged(KeyboardLayoutChangedStatusMessage { data: event })));
    });

    let s = sender.clone();
    listener.add_sub_map_changed_handler(move |data| {
        let _ = s.send(StatusEvent::Workspace(WorkspaceEvent::SubMapChanged(SubMapChangedStatusMessage { sub_map: data.into() })));
    });

    let s = sender.clone();
    listener.add_layer_opened_handler(move |data| {
        let _ = s.send(StatusEvent::Layer(LayerEvent::Opened(LayerOpenedStatusMessage { layer_name: data.into() })));
    });

    let s = sender.clone();
    listener.add_layer_closed_handler(move |data| {
        let _ = s.send(StatusEvent::Layer(LayerEvent::Closed(LayerClosedStatusMessage { layer_name: data.into() })));
    });

    let s = sender.clone();
    listener.add_float_state_changed_handler(move |data| {
        let event = HyprlandWindowFloatEventData {
            window_address: data.address.to_string().into(),
            is_floating: data.floating,
        };
        let _ = s.send(StatusEvent::Window(WindowEvent::FloatStateChanged(FloatStateChangedStatusMessage { data: event })));
    });

    let s = sender.clone();
    listener.add_urgent_state_changed_handler(move |data| {
        let _ = s.send(StatusEvent::Window(WindowEvent::UrgentStateChanged(UrgentStateChangedStatusMessage {
            window_address: data.to_string().into(),
        })));
    });

    let s = sender.clone();
    listener.add_window_title_changed_handler(move |data| {
        let event = HyprlandWindowTitleEventData {
            window_address: data.address.to_string().into(),
            window_title: data.title.into(),
        };
        let _ = s.send(StatusEvent::Window(WindowEvent::TitleChanged(WindowTitleChangedStatusMessage { data: event })));
    });

    let s = sender.clone();
    listener.add_workspace_renamed_handler(move |data| {
        let event = HyprlandNonSpecialWorkspaceData {
            workspace_name: data.name.into(),
            workspace_id: data.id,
        };
        let _ = s.send(StatusEvent::Workspace(WorkspaceEvent::Renamed(WorkspaceRenamedStatusMessage { data: event })));
    });

    let s = sender.clone();
    listener.add_special_removed_handler(move |data| {
        let _ = s.send(StatusEvent::Workspace(WorkspaceEvent::SpecialRemoved(SpecialRemovedStatusMessage {
            monitor_name: data.into(),
        })));
    });

    let s = sender.clone();
    listener.add_changed_special_handler(move |data| {
        let event = HyprlandChangedSpecialEventData {
            monitor_name: data.monitor_name.into(),
            special_workspace_name: data.workspace_name.into(),
        };
        let _ = s.send(StatusEvent::Workspace(WorkspaceEvent::ChangedSpecial(ChangedSpecialStatusMessage { data: event })));
    });

    let s = sender.clone();
    listener.add_screencast_handler(move |data| {
        let event = HyprlandScreencastEventData {
            screencast_type: if data.monitor {
                HyprlandScreencastType::Monitor
            } else {
                HyprlandScreencastType::Window
            },
            is_active: data.turning_on,
            owner: String::new().into(),
        };
        let _ = s.send(StatusEvent::System(SystemEvent::Screencast(ScreencastStatusMessage { data: event })));
    });

    let s = sender.clone();
    listener.add_config_reloaded_handler(move || {
        let _ = s.send(StatusEvent::System(SystemEvent::ConfigReloaded(ConfigReloadedStatusMessage {})));
    });

    let s = sender.clone();
    listener.add_ignore_group_lock_state_changed_handler(move |is_enabled| {
        let _ = s.send(StatusEvent::Group(GroupEvent::IgnoreLockChanged(IgnoreGroupLockStateChangedStatusMessage { is_enabled })));
    });

    let s = sender.clone();
    listener.add_lock_groups_state_changed_handler(move |is_locked| {
        let _ = s.send(StatusEvent::Group(GroupEvent::LockChanged(LockGroupsStateChangedStatusMessage { is_locked })));
    });

    let s = sender.clone();
    listener.add_window_pinned_handler(move |data| {
        let event = HyprlandWindowPinEventData {
            window_address: data.address.to_string().into(),
            is_pinned: data.pinned,
        };
        let _ = s.send(StatusEvent::Window(WindowEvent::Pinned(WindowPinnedStatusMessage { data: event })));
    });

    let s = sender.clone();
    listener.add_group_toggled_handler(move |data| {
        let event = HyprlandGroupToggledEventData {
            window_address: data.window_addresses.first().map(|a| a.to_string()).unwrap_or_default().into(),
            is_grouped: data.toggled,
        };
        let _ = s.send(StatusEvent::Group(GroupEvent::Toggled(GroupToggledStatusMessage { data: event })));
    });

    let s = sender.clone();
    listener.add_window_moved_into_group_handler(move |data| {
        let _ = s.send(StatusEvent::Group(GroupEvent::MovedInto(WindowMovedIntoGroupStatusMessage {
            window_address: data.to_string().into(),
        })));
    });

    let s = sender.clone();
    listener.add_window_moved_out_of_group_handler(move |data| {
        let _ = s.send(StatusEvent::Group(GroupEvent::MovedOut(WindowMovedOutOfGroupStatusMessage {
            window_address: data.to_string().into(),
        })));
    });

    listener.add_unknown_handler(move |data| {
        debug!("Hyprland: unknown event received: {:?}", data);
    });
}
