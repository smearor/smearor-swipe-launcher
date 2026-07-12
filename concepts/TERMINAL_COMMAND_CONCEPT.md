# Concept: Terminal Command Service (Service Plugin)

This concept describes the **Terminal Command Service** for the *Smearor Swipe Launcher*. It is closely related to the App Launcher, but instead of reading a
`.desktop` file and invoking `gio::DesktopAppInfo`, it executes an arbitrary command-line program directly. The command can be either an absolute path or a
binary resolvable via `$PATH`. The service also supports command arguments and environment variables.

Following the launcher’s decoupled architecture, the command execution logic lives in a singleton service crate, while the trigger is provided by a generic
widget (e.g., the `button` widget or a future `app_launcher` variant).

---

## 1. System Architecture & Data Flow

```
+---------------------------+                  +----------------------------+
| Button Widget (or any     |                  | Terminal Command Service   |
| compatible trigger)       |                  | (Central Singleton)        |
+---------------------------+                  +----------------------------+
            |                                              |
            |  1. Tap / Click (Launch)                     |
            |===========================================> |  2. Build process:
            |  Topic: "service.terminal_command.command" |     - Resolve command
            |  Payload: {                                |     - Apply args + env
            |    "action": "Launch",                     |     - Spawn detached
            |    "command_id": "rofi_run"                |
            |  }                                         |
            |                                            |
            |                                            |  3. Status Broadcast
            | <==========================================|     Topic: "service.terminal_command.status"
            |                                            |     Payload: {
            |                                            |       "command_id": "rofi_run",
            |                                            |       "status": "Running"
            |                                            |     }
```

---

## 2. Service Crate Layout

The new service crate is placed under `services/terminal_command` and is loaded as a shared library in `services.toml`.

### A. Crate Structure

```
services/terminal_command/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── service.rs
    └── config.rs
```

### B. Crate Registration

```toml
# services.toml
[[services]]
id = "terminal_command"
path = "target/release/libsmearor_terminal_command_service.so"
```

The service is registered under the topic prefix `service.terminal_command`.

---

## 3. Service Configuration

The service configuration defines the available commands. Each command is identified by a stable `command_id` that widgets reference when sending launch
requests.

```toml
# services.toml
[terminal_command]

[terminal_command.commands.rofi_run]
command = "rofi"
args = ["-show", "run"]
env = { "ROFI_THEME" = "/usr/share/rofi/themes/solarized" }

[terminal_command.commands.nmtui]
command = "/usr/bin/nmtui"
args = []
env = {}

[terminal_command.commands.custom_script]
command = "/home/user/.local/bin/launcher-helper.sh"
args = ["--toggle", "panel"]
env = { "LAUNCHER_MODE" = "swipe" }
```

### Configuration Fields

| Field                  | Type                  | Required | Description                                                                            |
|------------------------|-----------------------|----------|----------------------------------------------------------------------------------------|
| `command`              | `string`              | Yes      | Absolute path to a binary or a name resolvable via the current `$PATH`.                |
| `args`                 | `list<string>`        | No       | Ordered list of arguments passed to the command.                                       |
| `env`                  | `map<string, string>` | No       | Additional environment variables merged into the spawned process environment.          |
| `working_dir`          | `string`              | No       | Optional working directory for the spawned process.                                    |
| `restart_on_exit`      | `bool`                | No       | If `true`, the service restarts the command if it exits unexpectedly. Default `false`. |
| `kill_signal`          | `string`              | No       | Signal used for termination. Values: `"SIGTERM"` (default) or `"SIGKILL"`.             |
| `terminate_timeout_ms` | `u64`                 | No       | Grace period in milliseconds before escalating to `SIGKILL`. Default `2000`.           |

---

## 4. Message Topics & Payloads

### A. Command Topic: `service.terminal_command.command`

Widgets or other services send control messages to this topic.

#### Launch

```json
{
  "action": "Launch",
  "command_id": "rofi_run"
}
```

#### Terminate

```json
{
  "action": "Terminate",
  "command_id": "rofi_run"
}
```

#### Restart

```json
{
  "action": "Restart",
  "command_id": "rofi_run"
}
```

### B. Status Topic: `service.terminal_command.status`

The service broadcasts status updates for each tracked command.

```json
{
  "command_id": "rofi_run",
  "status": "Running",
  "pid": 12345
}
```

```json
{
  "command_id": "rofi_run",
  "status": "Stopped"
}
```

Status values:

- `Running`: The command has been spawned and is still active.
- `Stopped`: The command is no longer running (terminated, exited, or crashed).
- `Failed`: The command could not be started (e.g., binary not found, permission denied).

---

## 5. Service Implementation

### A. Internal State

```rust
pub struct TerminalCommandService {
    /// Configured commands, keyed by command_id.
    commands: Arc<DashMap<String, CommandDefinition>>,
    /// Currently tracked process handles, keyed by command_id.
    tracked_processes: Arc<DashMap<String, TrackedProcess>>,
}

pub struct CommandDefinition {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<PathBuf>,
    pub restart_on_exit: bool,
    pub kill_signal: KillSignal,
    pub terminate_timeout_ms: u64,
}

pub struct TrackedProcess {
    pub pid: u32,
    pub child: Child,
    pub started_at: Instant,
}
```

### B. Command Resolution

When launching a command:

1. Look up the `command_id` in the configured `commands` map.
2. If the configured `command` is an absolute path, use it directly.
3. If it is a relative name, resolve it via the current `$PATH` environment. The service must report `Failed` if the binary cannot be resolved.
4. Validate that the resolved path exists and is executable.

### C. Process Spawn

The service spawns the command as a detached child using `std::process::Command` or `tokio::process::Command`:

```rust
let child = tokio::process::Command::new(program)
    .args(&definition.args)
    .envs(&definition.env)
    .current_dir(definition.working_dir.as_deref().unwrap_or("."))
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()?;
```

After spawning:

1. Store the `Child` handle and PID in `tracked_processes`.
2. Broadcast `Running` on `service.terminal_command.status`.
3. Start a background task that waits for the child to exit. When it exits, remove the entry and broadcast `Stopped` (or `Failed` if the spawn itself failed).
   If `restart_on_exit` is enabled, the background task re-queues a `Launch` action.

### D. Termination

When receiving a `Terminate` action:

1. Look up the `command_id` in `tracked_processes`.
2. If no process is tracked, broadcast `Stopped` and return.
3. Send the configured signal (default `SIGTERM`) to the tracked PID.
4. Wait for `terminate_timeout_ms`.
5. If the process is still alive, send `SIGKILL`.
6. Remove the process from the tracking map and broadcast `Stopped`.

### E. Restart

A `Restart` action is equivalent to `Terminate` followed by `Launch` for the same `command_id`. It is atomic: the service must stop the running process before
starting a new one.

---

## 6. Background Process Reaping

Similar to the App Launcher service, a background task reaps tracked processes:

1. For each entry in `tracked_processes`, check whether the process is still alive.
2. If the process has exited, remove it from the map and broadcast `Stopped`.
3. If `restart_on_exit` is enabled for the command, schedule a new launch.

The reaper avoids stale entries and ensures the `button` widget or any subscriber receives accurate status updates.

---

## 7. Widget Integration

The service is designed to be triggered by generic widgets. The simplest integration is the existing `button` widget:

```toml
# config.toml
[[scroll_band.plugins]]
id = "run_rofi"
path = "target/release/libsmearor_button_widget.so"

[run_rofi]
text = "Run"
icon = "nf-md-terminal"
click_topic = "service.terminal_command.command"
click_payload = { action = "Launch", command_id = "rofi_run" }
```

If a widget wants to reflect the running state, it subscribes to `service.terminal_command.status` and filters by its own `command_id`.

---

## 8. Security & Best Practices

* **No shell interpretation:** The service must pass arguments directly to the executable without invoking a shell. This prevents injection attacks from
  widget-supplied payload values.
* **Command whitelist:** Commands are fully defined in `services.toml`, not by widgets. Widgets can only reference pre-configured `command_id` values.
* **Validation:** The service must reject `Launch` requests with an unknown `command_id` and broadcast a `Failed` status.
* **Sanitization of env variables:** The service passes the configured environment map verbatim. It must not merge untrusted data from widget payloads into the
  environment.
* **Working directory:** If `working_dir` is configured, the service must validate that it exists and is a directory before spawning.

---

## 9. Advantages Over `.desktop` Launch

* **Flexibility:** Commands do not require a `.desktop` file. This is useful for helper scripts, terminal UI tools, or one-off binaries.
* **Argument support:** Arguments are explicitly configured, avoiding the placeholder handling required by `.desktop` `Exec` lines.
* **Environment control:** Each command can receive a tailored environment without wrapper shell scripts.
* **Termination support:** Long-running commands can be terminated via the same message interface, enabling the same widget patterns as the App Launcher.

---

## 10. Exit Criteria

The concept is fully implemented when:

* `services/terminal_command` exists as a standalone crate following the project’s service plugin conventions.
* The service is loadable via `services.toml` and implements the required `Service` traits.
* It can launch a configured command from either an absolute path or `$PATH`.
* It supports arguments and environment variables as configured.
* It broadcasts correct `Running`/`Stopped`/`Failed` status messages.
* It can terminate and restart tracked processes.
* A `button` widget can trigger a launch using a `click_payload` reference.
