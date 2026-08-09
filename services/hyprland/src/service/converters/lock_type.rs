use hyprland::dispatch::LockType;
use smearor_hyprland_model::HyprlandLockType;

pub(crate) fn convert_lock_type(lt: HyprlandLockType) -> LockType {
    match lt {
        HyprlandLockType::Lock => LockType::Lock,
        HyprlandLockType::Unlock => LockType::Unlock,
        HyprlandLockType::ToggleLock => LockType::ToggleLock,
    }
}
