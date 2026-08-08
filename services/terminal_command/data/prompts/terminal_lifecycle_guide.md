Terminal command lifecycle guide:

1. BEFORE LAUNCH: Always read terminal_command://configured to verify the command_id exists.
2. LAUNCH: Use terminal_command_launch with the correct command_id. Check forked and terminate_on_exit parameters.
3. MONITOR: Read terminal_command://running to check if the process is still active.
4. TERMINATE: Use terminal_command_terminate to stop a running command by command_id.
5. RESTART: Use terminal_command_restart to terminate and relaunch a command.

Important: Never launch a command without verifying it exists in the configuration first. If a command has restart_on_exit=true, it will automatically restart
after termination.
