use smearor_power_model::PowerAction;
use smearor_power_model::PowerStatusMessage;
use smearor_power_model::ScheduledActionInfo;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Notify;
use tracing::debug;

/// Shared state for the power service.
pub struct PowerState {
    /// The current status message.
    pub status: PowerStatusMessage,
}

/// Runs a countdown timer before executing a power action.
///
/// Each second, the state is updated with the remaining countdown time and
/// the updated status is sent via the provided sender. If the cancel token
/// is triggered, the countdown is aborted.
pub async fn run_countdown(
    action: PowerAction,
    seconds: u32,
    state: Arc<Mutex<PowerState>>,
    status_sender: tokio::sync::mpsc::UnboundedSender<PowerStatusMessage>,
    cancel_token: Arc<Notify>,
    connection: zbus::Connection,
    lock_command: String,
    logout_command: String,
) {
    debug!("Power Service: starting countdown for {action:?}, {seconds}s");
    for remaining in (1..=seconds).rev() {
        tokio::select! {
            _ = cancel_token.notified() => {
                debug!("Power Service: countdown cancelled");
                if let Ok(mut current) = state.lock() {
                    current.status.countdown_active = false;
                    current.status.countdown_remaining_seconds = 0;
                    let cancelled_status = current.status.clone();
                    drop(current);
                    let _ = status_sender.send(cancelled_status);
                }
                return;
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                if let Ok(mut current) = state.lock() {
                    current.status.countdown_active = true;
                    current.status.countdown_remaining_seconds = remaining;
                    current.status.countdown_action = action.clone();
                    let status = current.status.clone();
                    drop(current);
                    let _ = status_sender.send(status);
                }
            }
        }
    }

    debug!("Power Service: countdown expired, executing action {action:?}");
    crate::dbus::execute_power_action(&connection, &action, &lock_command, &logout_command).await;
    if let Ok(mut current) = state.lock() {
        current.status.countdown_active = false;
        current.status.countdown_remaining_seconds = 0;
        let status = current.status.clone();
        drop(current);
        let _ = status_sender.send(status);
    }
}

/// Runs a scheduled action timer that executes after the specified delay.
///
/// Each second, the state is updated with the remaining time and the updated
/// status is sent via the provided sender. If the cancel token is triggered,
/// the scheduled action is aborted.
pub async fn run_scheduled_action(
    action: PowerAction,
    delay_seconds: u64,
    state: Arc<Mutex<PowerState>>,
    status_sender: tokio::sync::mpsc::UnboundedSender<PowerStatusMessage>,
    cancel_token: Arc<Notify>,
    connection: zbus::Connection,
    lock_command: String,
    logout_command: String,
) {
    debug!("Power Service: scheduling {action:?} in {delay_seconds}s");
    let start = Instant::now();
    loop {
        let elapsed = start.elapsed().as_secs();
        let remaining = delay_seconds.saturating_sub(elapsed);

        if remaining == 0 {
            debug!("Power Service: scheduled action timer expired, executing {action:?}");
            crate::dbus::execute_power_action(&connection, &action, &lock_command, &logout_command).await;
            if let Ok(mut current) = state.lock() {
                current.status.scheduled_action = stabby::option::Option::None();
                let status = current.status.clone();
                drop(current);
                let _ = status_sender.send(status);
            }
            return;
        }

        tokio::select! {
            _ = cancel_token.notified() => {
                debug!("Power Service: scheduled action cancelled");
                if let Ok(mut current) = state.lock() {
                    current.status.scheduled_action = stabby::option::Option::None();
                    let cancelled_status = current.status.clone();
                    drop(current);
                    let _ = status_sender.send(cancelled_status);
                }
                return;
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                if let Ok(mut current) = state.lock() {
                    current.status.scheduled_action = stabby::option::Option::Some(ScheduledActionInfo {
                        action: action.clone(),
                        remaining_seconds: remaining,
                        total_delay_seconds: delay_seconds,
                    });
                    let status = current.status.clone();
                    drop(current);
                    let _ = status_sender.send(status);
                }
            }
        }
    }
}
