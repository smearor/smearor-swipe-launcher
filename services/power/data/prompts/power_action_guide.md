Tools:

- system_power_action: Execute a power action immediately (shutdown, reboot, suspend, hibernate, lock, logout)
- system_schedule_power_action: Schedule a shutdown or reboot in N minutes
- system_cancel_power_action: Cancel a running countdown or scheduled action
- system_reboot_to_uefi: Reboot directly into BIOS/UEFI

Resources:

- power://capabilities: System power capabilities
- power://inhibitors: Active inhibitor locks
- power://scheduled_actions: Currently scheduled power action

Safety: Always check inhibitors before executing power actions. Warn the user about unsaved work.
