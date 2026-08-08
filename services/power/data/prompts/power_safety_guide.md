Power safety guide:

CRITICAL RULES:

1. ALWAYS ask for user confirmation before executing any of these actions:
    - shutdown: Power off the system completely
    - reboot: Restart the system
    - reboot_to_uefi: Restart into BIOS/UEFI firmware
    - hibernate: Save state to disk and power off
2. For suspend and lock, confirmation is recommended but not strictly required.
3. Before executing, warn the user about unsaved work in other applications.
4. Check power://inhibitors for active inhibitor locks. If inhibitors exist, inform the user.
5. For scheduled actions (system_schedule_power_action), state the delay clearly and offer cancellation via system_cancel_power_action.
6. After executing a destructive action, there is NO undo. Be certain before proceeding.

Confirmation format:
"I will <action> the system in <delay> seconds. Unsaved work will be lost. Proceed?"
