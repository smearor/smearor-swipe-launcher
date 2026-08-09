use crate::event_listener::listener::HyprlandEvent;
use crate::service::HyprlandSharedState;
use crate::status::RATE_LIMIT_MS;
use crate::status::RateLimiter;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;

/// Thin dispatch loop: routes events to domain workers. No processing logic here.
/// Uses `tokio::select!` to simultaneously wait for incoming events and a periodic
/// flush interval for the rate limiter's trailing-edge debounce.
pub fn spawn_event_worker(
    mut event_receiver: mpsc::UnboundedReceiver<HyprlandEvent>,
    core_context: Option<FfiCoreContext>,
    meta: PluginMeta,
    enable_workspace_lifecycle: bool,
    shared_state: Arc<Mutex<HyprlandSharedState>>,
) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(err) => {
                tracing::error!("Hyprland event worker: failed to create runtime: {err}");
                return;
            }
        };
        rt.block_on(async move {
            let mut workspace_state = crate::workspace::WorkspaceState::new();
            let mut status_rate_limiter = RateLimiter::new();

            let mut flush_interval = tokio::time::interval(Duration::from_millis(RATE_LIMIT_MS));
            flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    maybe_event = event_receiver.recv() => {
                        let Some(event) = maybe_event else { break; };
                        match event {
                            HyprlandEvent::Workspace(e) => {
                                crate::workspace::process_event(
                                    &mut workspace_state, e, &core_context, &meta, enable_workspace_lifecycle, &shared_state,
                                ).await;
                            }
                            HyprlandEvent::Monitor(e) => {
                                crate::monitor::process_event(e, &core_context, &meta, &shared_state).await;
                            }
                            HyprlandEvent::Status(e) => {
                                status_rate_limiter.process_event(e, &core_context, &meta, &shared_state);
                            }
                        }
                    }
                    _ = flush_interval.tick() => {
                        while let Some(pending) = status_rate_limiter.flush_trailing() {
                            RateLimiter::broadcast_event(&core_context, &meta, pending);
                        }
                    }
                }
            }
        });
    });
}
