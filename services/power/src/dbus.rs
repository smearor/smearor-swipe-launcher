use smearor_power_model::InhibitorInfo;
use smearor_power_model::PowerAction;
use smearor_power_model::PowerCapabilities;
use tracing::debug;
use tracing::error;
use zbus::Connection;

/// D-Bus proxy for `org.freedesktop.login1.Manager`.
#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait LoginManager {
    fn power_off(&self, interactive: bool) -> zbus::Result<()>;
    fn reboot(&self, interactive: bool) -> zbus::Result<()>;
    fn suspend(&self, interactive: bool) -> zbus::Result<()>;
    fn hibernate(&self, interactive: bool) -> zbus::Result<()>;
    fn can_power_off(&self) -> zbus::Result<String>;
    fn can_reboot(&self) -> zbus::Result<String>;
    fn can_suspend(&self) -> zbus::Result<String>;
    fn can_hibernate(&self) -> zbus::Result<String>;
    fn can_reboot_to_firmware(&self) -> zbus::Result<String>;
    fn set_reboot_to_firmware(&self, enable: bool) -> zbus::Result<()>;
    fn list_inhibitors(&self) -> zbus::Result<Vec<(String, String, String, String, u32, u32)>>;
}

/// D-Bus proxy for `org.freedesktop.login1.Session`.
#[zbus::proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto"
)]
trait LoginSession {
    fn lock(&self) -> zbus::Result<()>;
    fn terminate(&self) -> zbus::Result<()>;
}

/// Queries all capability flags from systemd-logind.
pub async fn refresh_capabilities(connection: &Connection) -> PowerCapabilities {
    let manager = LoginManagerProxy::new(connection).await;
    match manager {
        Ok(proxy) => PowerCapabilities {
            can_shutdown: proxy.can_power_off().await.unwrap_or_default() == "yes",
            can_reboot: proxy.can_reboot().await.unwrap_or_default() == "yes",
            can_suspend: proxy.can_suspend().await.unwrap_or_default() == "yes",
            can_hibernate: proxy.can_hibernate().await.unwrap_or_default() == "yes",
            can_reboot_to_firmware: proxy.can_reboot_to_firmware().await.unwrap_or_default() == "yes",
            can_lock: true,
            can_logout: true,
        },
        Err(e) => {
            error!("Power Service: failed to create login1 manager proxy: {e}");
            PowerCapabilities::default()
        }
    }
}

/// Queries all active inhibitor locks from systemd-logind.
pub async fn refresh_inhibitors(connection: &Connection) -> Vec<InhibitorInfo> {
    let manager = LoginManagerProxy::new(connection).await;
    match manager {
        Ok(proxy) => {
            let raw = proxy.list_inhibitors().await.unwrap_or_default();
            raw.into_iter()
                .filter(|(what, _, _, _, _, _)| {
                    let what = what.to_lowercase();
                    what.contains("shutdown") || what.contains("reboot")
                })
                .map(|(what, who, why, process_name, _, _)| InhibitorInfo {
                    what: stabby::string::String::from(what),
                    who: stabby::string::String::from(who),
                    reason: stabby::string::String::from(why),
                    process_name: stabby::string::String::from(process_name),
                })
                .collect()
        }
        Err(e) => {
            error!("Power Service: failed to query inhibitors: {e}");
            Vec::new()
        }
    }
}

/// Executes a power action via D-Bus calls to systemd-logind.
pub async fn execute_power_action(connection: &Connection, action: &PowerAction, lock_command: &str, logout_command: &str) {
    match action {
        PowerAction::Shutdown => {
            if let Err(e) = execute_power_off(connection).await {
                error!("Power Service: failed to power off: {e}");
            }
        }
        PowerAction::Reboot => {
            if let Err(e) = execute_reboot(connection).await {
                error!("Power Service: failed to reboot: {e}");
            }
        }
        PowerAction::Suspend => {
            if let Err(e) = execute_suspend(connection).await {
                error!("Power Service: failed to suspend: {e}");
            }
        }
        PowerAction::Hibernate => {
            if let Err(e) = execute_hibernate(connection).await {
                error!("Power Service: failed to hibernate: {e}");
            }
        }
        PowerAction::Lock => {
            if lock_command.is_empty() {
                if let Err(e) = execute_session_lock(connection).await {
                    error!("Power Service: failed to lock session: {e}");
                }
            } else {
                debug!("Power Service: executing custom lock command: {lock_command}");
                spawn_custom_command(lock_command);
            }
        }
        PowerAction::Logout => {
            if logout_command.is_empty() {
                if let Err(e) = execute_session_terminate(connection).await {
                    error!("Power Service: failed to terminate session: {e}");
                }
            } else {
                debug!("Power Service: executing custom logout command: {logout_command}");
                spawn_custom_command(logout_command);
            }
        }
        PowerAction::RebootToFirmware => {
            if let Err(e) = execute_reboot_to_firmware(connection).await {
                error!("Power Service: failed to reboot to firmware: {e}");
            }
        }
        PowerAction::Cancel => {
            debug!("Power Service: cancel action requested, no D-Bus call needed");
        }
    }
}

async fn execute_power_off(connection: &Connection) -> zbus::Result<()> {
    let proxy = LoginManagerProxy::new(connection).await?;
    proxy.power_off(false).await
}

async fn execute_reboot(connection: &Connection) -> zbus::Result<()> {
    let proxy = LoginManagerProxy::new(connection).await?;
    proxy.reboot(false).await
}

async fn execute_suspend(connection: &Connection) -> zbus::Result<()> {
    let proxy = LoginManagerProxy::new(connection).await?;
    proxy.suspend(false).await
}

async fn execute_hibernate(connection: &Connection) -> zbus::Result<()> {
    let proxy = LoginManagerProxy::new(connection).await?;
    proxy.hibernate(false).await
}

async fn execute_session_lock(connection: &Connection) -> zbus::Result<()> {
    let proxy = LoginSessionProxy::new(connection).await?;
    proxy.lock().await
}

async fn execute_session_terminate(connection: &Connection) -> zbus::Result<()> {
    let proxy = LoginSessionProxy::new(connection).await?;
    proxy.terminate().await
}

async fn execute_reboot_to_firmware(connection: &Connection) -> zbus::Result<()> {
    let proxy = LoginManagerProxy::new(connection).await?;
    proxy.set_reboot_to_firmware(true).await?;
    proxy.reboot(false).await
}

fn spawn_custom_command(command: &str) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }
    let program = parts[0];
    let args = &parts[1..];
    if let Err(e) = std::process::Command::new(program).args(args).spawn() {
        error!("Power Service: failed to spawn custom command '{command}': {e}");
    }
}
