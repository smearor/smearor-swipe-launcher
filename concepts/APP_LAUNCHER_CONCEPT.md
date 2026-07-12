# Concept: App Launcher (Widget & Service Plugin)

This concept describes the full implementation of the **App Launcher** in the *Smearor Swipe Launcher*. In accordance with our decoupled architecture, the
system is strictly split into two independent components:

1. **The App Launcher Widget Plugin:** A pure UI component (View), of which multiple instances with different application configurations can be integrated into
   the launcher UI.
2. **The App Launcher Service Plugin:** A background singleton service (Logic/Controller) managing process execution, PID tracking, and POSIX signal handling.

---

## 1. System Architecture & Data Flow

```
+--------------------------+                 +----------------------------+
| App Launcher Widget      |                 | App Launcher Service       |
| (Instance for Firefox)   |                 | (Central Singleton)        |
+--------------------------+                 +----------------------------+
             |                                             |
             |  1. Tap / Click (Launch)                    |
             |===========================================> |  2. GIO / procfs:
             |  Topic: "service.app_launcher.command"      |     - Parse .desktop
             |  Payload: {                                 |     - Command spawn
             |    "action": "Launch",                      |     - PID tracking
             |    "desktop_file": ".../firefox.desktop"    |
             |  }                                          |
             |                                             |
             |                                             |  3. Status Broadcast (Running)
             | <===========================================|     Topic: "service.app_launcher.status"
             |                                             |     Payload: {
             |                                             |       "desktop_file": ".../firefox.desktop",
             |                                             |       "status": "Running"
             |                                             |     }
             |                                             |
             |  4. Visual Update:                          |
             |     Activates LED indicator                 |
```

---

## 2. The Widget Plugin (Pure View)

The widget is completely stateless. It is parameterized via `config.toml` with the path to the `.desktop` file.

### A. Widget Configuration

```toml
# config.toml
[[scroll_band.plugins]]
id = "launcher_firefox"
path = "target/debug/libapp_launcher_widget.so"

[launcher_firefox]
desktop_file_path = "/usr/share/applications/firefox.desktop"
```

### B. Functionality (View Lifecycle)

1. **Initialization:** Upon loading, the widget parses the `.desktop` file once using GNOME's `gio::DesktopAppInfo` interface to extract the **Display Name** (
   e.g., `"Firefox Web Browser"`) and the **Icon Name** (e.g., `"firefox"`).
2. **UI Rendering (GTK4):**
    * Renders a square tile (e.g., 96x96 pixels).
    * The icon is resolved via the local GTK icon theme (e.g., Yaru) and drawn centered.
    * The app name is placed legibly below the icon.
    * In the top-right corner, a small, initially invisible LED dot serves as the "Running" indicator.
3. **Message Subscription:**
   At startup, the widget subscribes to the topic `service.app_launcher.status`.

### C. Interactions & Event Publishing

#### Click / Tap (Launch App):

Triggers an asynchronous launch command sent to the service:

```json
Topic:   "service.app_launcher.command"
Payload: {
"action": "Launch",
"desktop_file": "/usr/share/applications/firefox.desktop"
}
```

#### Longpress / Right Click (Terminate App):

Triggers a termination command sent to the service:

```json
Topic:   "service.app_launcher.command"
Payload: {
"action": "Terminate",
"desktop_file": "/usr/share/applications/firefox.desktop"
}
```

#### Status Feedback Loop:

When a status message is received on `service.app_launcher.status`, the widget evaluates the JSON payload:

* Does the received `desktop_file` path match its own configuration?
    * Yes, and `status == "Running"`: LED indicator turns green.
    * Yes, and `status == "Stopped"`: LED indicator turns off.
    * No: Message is discarded (this status change concerns another app instance).

---

## 3. The Service Plugin (Pure Logic)

The service is loaded once as a singleton in the background. It listens exclusively to control commands and has zero dependency on GTK.

### A. Internal State

```rust
pub struct AppLauncherService {
    // Maps a .desktop file path to a list of currently running PIDs
    pub tracked_processes: Arc<DashMap<String, Vec<u32>>>,
}
```

### B. Command Handler (on_message)

The service processes incoming `FfiEnvelope` events on the `service.app_launcher.command` topic.

#### 1. Executing "Launch" Command

When the service receives a launch command:

1. It opens the specified `.desktop` file and parses the `Exec` line via `gio::DesktopAppInfo`.
2. **Security Cleaning:** It filters out placeholders (such as `%u`, `%U`, `%f`, `%F`) from the execution command to prevent shell injection or syntax issues.
3. It spawns the process as a **detached child** using `std::process::Command`:
   ```rust
   let child = Command::new(program)
       .args(clean_args)
       .stdin(Stdio::null())
       .stdout(Stdio::null())
       .stderr(Stdio::null())
       .spawn()?;
   ```
4. It registers the newly spawned PID under the entry for this `desktop_file` in its state map.
5. It broadcasts a status event on `service.app_launcher.status`:
   ```json
   {
     "desktop_file": "/usr/share/applications/firefox.desktop",
     "status": "Running"
   }
   ```

#### 2. Executing "Terminate" Command (SIGTERM)

When the service receives a termination command:

1. It retrieves all tracked PIDs mapped to the specified `desktop_file`.
2. It verifies via `/proc/<pid>` if the processes are still active in the Linux system (avoiding killing recycled PIDs).
3. It sends the `SIGTERM` signal (graceful termination) to all active PIDs:
   ```rust
   nix::sys::signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM)?;
   ```
4. If a process persists after a timeout (e.g., 2 seconds), it may optionally send `SIGKILL` (forceful termination).
5. The PIDs are removed from the tracking list.
6. It broadcasts a status event on `service.audio.status`:
   ```json
   {
     "desktop_file": "/usr/share/applications/firefox.desktop",
     "status": "Stopped"
   }
   ```

---

## 4. Background Process Reaping (GC Loop)

Since applications can be closed directly by the user (e.g., by clicking the close button on the Firefox window), PIDs would remain in the service state map as
orphaned entries, keeping the widget LED illuminated incorrectly.

### The GC Check (Garbage Collector Loop)

Upon initialization, the service starts an asynchronous background task (spawned via `tokio::spawn`):

1. **Loop every 2 seconds:** Iterates over all tracked desktop file paths and their associated PIDs.
2. For each PID, it verifies the existence of `/proc/<pid>`.
3. If `/proc/<pid>` does not exist, the process has exited. The service removes the PID from its map.
4. If the PID list for a specific desktop file becomes empty (all windows closed), it broadcasts the "Stopped" status update:
   ```json
   Topic: "service.app_launcher.status"
   Payload: { "desktop_file": "...", "status": "Stopped" }
   ```
   The corresponding widgets then automatically turn off their green LED indicators.

---

## 5. Architectural Advantages

* **High Modularity:** Changes to launcher UI sizing or design only affect the widget plugin. The process tracking service remains untouched.
* **Consolidated Background Checks:** Checking `/proc` is grouped into a single asynchronous loop inside the service, rather than spawning 10 separate
  background threads for 10 individual widget instances.
* **Trivial Integration:** Other system widgets (e.g., a bottom task bar or active window switcher) can easily subscribe to the `service.app_launcher.status`
  topic to receive real-time state without any duplicate process checking logic.