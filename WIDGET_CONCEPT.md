# Widget Concept

Dieses Konzept für die Plugin-Architektur des *Smearor Swipe Launchers* setzt auf die ABI-Stabilität des Rust-Crates und eine klare Trennung zwischen
Widget-UI (Rendering) und Logik (Backend-Anbindung an D-Bus/System-APIs).

---

### Roadmap: Implementierungsphasen

#### Phase 1: Core & Infrastruktur (Fundament)

* **ABI-Definition:** Finalisierung der Crate-Struktur (FFI-sichere Schnittstellen).
* **Event-Bus:** Implementierung eines internen Messaging-Systems, damit Widgets auf globale System-Events (z. B. Desktop-Umgebungs-Signale) reagieren können.
* **Proxy-Schicht:** Entwicklung eines Sockets oder Shared-Memory-Bereichs für die Kommunikation zwischen Plugin-Prozessen und Launcher-Hauptprozess.

#### Phase 2: App & Audio (Interaktions-Fokus)

* **App Launcher:** Nutzung von `gio` oder `glib` unter Linux, um `.desktop`-Dateien zu parsen. Implementierung der PID-Verwaltung (Tracking via `/proc`).
* **Audio Widget:** Integration von `libpulse` oder `pipewire` (via `libpipewire`). Mapping von Touch-Gesten (Swipe up/down) auf Lautstärke-Schritte.

#### Phase 3: Connectivity & Media (System-Status)

* **Network/Bluetooth:** Anbindung an `NetworkManager` (D-Bus) und `BlueZ` (D-Bus).
* **MPRIS:** Implementierung des MPRIS2-D-Bus-Interfaces. Fokus auf die Abfrage von `org.mpris.MediaPlayer2.Player` (Metadaten-Caching für Album-Cover).

#### Phase 4: System-Services (Komplexität)

* **Notifications:** Implementierung des `org.freedesktop.Notifications` D-Bus-Interfaces (Server-Modus). Aufbau eines lokalen Stacks zur Verwaltung der
  History.

---

### Technisches Konzept der Widgets

| Widget            | Primäre Datenquelle / API   | Besondere Anforderung                                 |
|-------------------|-----------------------------|-------------------------------------------------------|
| **App Launcher**  | `gio::DesktopAppInfo`       | PID-Tracking, Signal-Handling (SIGTERM bei Longpress) |
| **Audio**         | `libpipewire` / `libpulse`  | Swipe-Event-Mapping (Delta-Berechnung)                |
| **Network**       | `NetworkManager` D-Bus API  | Status-Polling oder Signal-Listener                   |
| **Bluetooth**     | `BlueZ` (D-Bus)             | Pairing-State-Tracking                                |
| **MPRIS**         | D-Bus (MPRIS2 Spec)         | Thumbnail-Download / Caching von Album-Art            |
| **Notifications** | D-Bus (Notification Server) | Queue-Management & Timeout-Handling                   |

---

### Detail-Implementierung (Beispiele)

#### App Launcher Widget (Logik-Ansatz)

* **Konfiguration:** Ein Plugin-spezifisches Konfigurationsfeld für den Pfad zur `.desktop`-Datei.
* **Lifecycle:**

1. `exec()` call -> `gio::AppInfo::launch()`.
2. `Child`-Prozess wird als Hashmap im Plugin-Zustand gespeichert (`Map<Path, Pid>`).
3. **Longpress-Logik:** Bei Erkennung des Longpress-Events wird der gespeicherte PID-Wert via `nix::sys::signal::kill(pid, Signal::SIGTERM)` angesprochen.

#### Audio Widget (Interaktion)

* **UI-Feedback:** Dynamische Anzeige des Lautstärke-Balkens.
* **Gesten-Verarbeitung:**
* `SwipeUp`: `volume += delta`.
* `SwipeDown`: `volume -= delta`.
* `Tap`: Toggle Mute-Status.


* **Linux-Integration:** Verwendung von `pipewire` bietet die modernste Schnittstelle für eine niedrige Latenz bei der Lautstärkeanpassung.

#### MPRIS Widget (Datenfluss)

1. **Discovery:** Scan nach D-Bus Objekten unter `org.mpris.MediaPlayer2.*`.
2. **Mapping:**

* `PlaybackStatus` -> Play/Pause Icons.
* `Metadata` -> Zugriff auf `xesam:title`, `xesam:artist` und `mpris:artUrl`.


3. **Cover-Handling:** Das Widget sollte die URL im Cache ablegen, um Performance-Einbrüche beim Scrollen zu vermeiden.

---

### Empfehlung für die Plugin-API

Damit die Widgets unter Ubuntu/Linux performant bleiben, sollte das Crate folgende Traits für Plugin-Entwickler bereitstellen:

```rust
trait SmearorWidget {
    fn update(&mut self); // Zyklisches Update
    fn on_touch(&mut self, event: TouchEvent); // Geste (Swipe, Tap, Longpress)
    fn render(&self) -> FrameData; // Übergabe an den Renderer
}

```

Diese Struktur ermöglicht es externen Entwicklern, Widgets ohne Kenntnis des Haupt-Launchers zu bauen, solange das ABI-stabile Binary die geforderten Methoden
exportiert. Da Ubuntu/Linux als Basis dient, ist die Verwendung von D-Bus als universelles Kommunikationsmittel für alle Netzwerk-, Bluetooth- und Media-Widgets
zwingend, da dies die native Art der Interaktion mit Gnome- und System-Komponenten ist.