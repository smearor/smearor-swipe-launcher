# Service-Plugin Konzept (SOA-Architektur)

Dieses Dokument beschreibt das Konzept für ein entkoppeltes **Service-Plugin-System** im *Smearor Swipe Launcher*. Es erweitert das bestehende Widget-System um
eine **Service-Oriented Architecture (SOA)** über die ABI-stabile FFI-Schnittstelle.

---

## 1. Das Problem & Die Motivation

Bisher waren Widgets im *Smearor Swipe Launcher* monolithisch aufgebaut: Sie hielten sowohl den visuellen GTK-Zustand als auch die System-Logik (z. B.
Prozess-Verwaltung, D-Bus-Listener oder Sockets).

Das führt zu gravierenden Problemen bei der Skalierbarkeit und Ressourceneffizienz:

* **Mehrfach-Instanziierung von Logik:** Wird ein App-Launcher-Widget für 5 unterschiedliche Applikationen in die `config.toml` eingebunden, werden 5 getrennte
  Plugin-Instanzen geladen. Jede Instanz müsste dieselbe PID-Verwaltung und dieselben Hintergrund-Threads starten.
* **Race Conditions bei Hardware-Ressourcen:** Wenn mehrere Audio-Widgets (z. B. ein Slider im Hauptmenü, ein Mute-Button in der Statusleiste) getrennt
  Verbindungen zu PipeWire aufbauen, kommt es zu Sync-Konflikten und unnötiger Last.
* **Kein globaler State:** Ein Widget weiß nichts über den Zustand des anderen.

### Die Lösung: Trennung von View und Service

Wir trennen die Widgets (UI-Darstellung) strikt von den Services (System-Logik, Hardware-Schnittstellen):

1. **Widgets (View-Only):** Reine, zustandslose UI-Kacheln. Sie nehmen Touch-Gesten entgegen, übersetzen diese in logische Nachrichten (Events) und schicken sie
   an den zentralen Event-Broker. Sie zeichnen sich rein basierend auf Status-Nachrichten, die sie vom Event-Broker erhalten.
2. **Services (Logic-Only):** Singletons, die im Hintergrund laufen, keinen GTK-Widget-Code enthalten und sich um System-Aufgaben kümmern. Sie lauschen auf
   Steuerungs-Events von Widgets und senden Status-Broadcasts an alle registrierten Abonnenten.

---

## 2. Systemarchitektur & Nachrichtenfluss

```
+-------------------------------------------------------------+
|                     CORE APPLICATION                        |
|                                                             |
|   +-----------------------------------------------------+   |
|   |                  Central Event Broker               |   |
|   +-----------------------------------------------------+   |
|             ^                                 |             |
|    publish  | (MPSC)                  publish | (Broadcast) |
|             |                                 v             |
+-------------|---------------------------------|-------------+
              |                                 |
      [ FfiEnvelope ]                   [ FfiEnvelope ]
              |                                 |
+-------------|-------------+     +-------------v-------------+
|    WIDGET-PLUGINS (UI)    |     |    SERVICE-PLUGINS (LOGIC)|
|                           |     |                           |
| +-----------------------+ |     | +-----------------------+ |
| | App-Launcher Widget 1 | |     | | ApplicationLauncher   | |
| | (Firefox Button)      | |     | | Service (Singleton)   | |
| +-----------------------+ |     | +-----------------------+ |
|                           |     |                           |
| +-----------------------+ |     | +-----------------------+ |
| | App-Launcher Widget 2 | |     | | AudioVolumeControl    | |
| | (Terminal Button)     | |     | | Service (Singleton)   | |
| +-----------------------+ |     | +-----------------------+ |
+---------------------------+     +---------------------------+
```

---

## 3. ABI-Schnittstelle für Service-Plugins

Wir führen ein neues FFI-stables VTable-Layout für Services im `smearor-plugin-api` Crate ein.

### A. Die Service-VTable (`smearor-plugin-api/src/service.rs`)

```rust
use crate::FfiCoreContext;
use crate::FfiEnvelope;
use abi_stable::RRef;
use abi_stable::StableAbi;
use abi_stable::derive_macro_reexports::RResult;
use abi_stable::std_types::RString;

#[repr(C)]
#[derive(StableAbi)]
pub struct ServiceVTable {
    pub destroy: unsafe extern "C" fn(service: *mut ()),
    pub get_id: unsafe extern "C" fn(service: *mut ()) -> RString,
    pub get_display_name: unsafe extern "C" fn(service: *mut ()) -> RString,
    pub on_message: unsafe extern "C" fn(service: *mut (), message: FfiEnvelope),
}

#[repr(C)]
#[derive(StableAbi)]
pub struct LoadedService {
    pub service_instance: *mut (),
    pub vtable: RRef<'static, ServiceVTable>,
}

pub type ServiceConstructor = unsafe extern "C" fn(
    config_json: *const i8,
    config_len: usize,
    core_context: FfiCoreContext,
) -> RResult<LoadedService, RString>;
```

---

## 4. Detaillierte Fallbeispiele

### Beispiel 1: ApplicationLauncher-Service & Multiplex-Widgets

#### 1. Die Widgets (z. B. 5 Instanzen)

Jedes App-Launcher-Widget wird mit einer einfachen Konfiguration gestartet (z. B. Pfad zur Desktop-Datei). Es rendert das Icon und einen Kreis-Indikator, ob die
App läuft.

* **Interaktion (Klick):**
  Sendet Nachricht an den Broker:
  ```json
  Topic:   "service.app_launcher.command"
  Payload: { "action": "Launch", "desktop_file": "/usr/share/applications/firefox.desktop" }
  ```
* **Interaktion (Longpress):**
  Sendet Nachricht an den Broker:
  ```json
  Topic:   "service.app_launcher.command"
  Payload: { "action": "Terminate", "desktop_file": "/usr/share/applications/firefox.desktop" }
  ```

#### 2. Der Service (ApplicationLauncher, Singleton)

* **Start:** Wird beim Anwendungsstart genau *einmal* geladen und instanziiert. Er lauscht auf das Topic `service.app_launcher.command`.
* **Zustand:** Hält eine interne `HashMap<String, Vec<u32>>` (Desktop-Pfad -> Liste aktiver PIDs).
* **Abarbeitung von "Launch":**
    1. Startet das Programm via `Command::new` entkoppelt im Hintergrund.
    2. Merkt sich die PID in der Map für diesen Desktop-Pfad.
    3. Publiziert Status-Update auf Topic `service.app_launcher.status`:
         ```json
         Payload: { "desktop_file": "/usr/share/applications/firefox.desktop", "status": "Running" }
         ```
* **Abarbeitung von "Terminate":**
    1. Holt alle PIDs für den Desktop-Pfad aus der Map.
    2. Sendet `SIGTERM` (und nach Timeout `SIGKILL`) an alle PIDs.
    3. Validiert über `/proc`, ob sie beendet wurden.
    4. Publiziert Status-Update:
       ```json
       Payload: { "desktop_file": "/usr/share/applications/firefox.desktop", "status": "Stopped" }
       ```
* **Status-Feedback an Widgets:**
  Sämtliche App-Launcher-Widgets lauschen auf `service.app_launcher.status`. Erhält ein Widget eine Nachricht, deren `desktop_file` mit seinem eigenen Pfad
  übereinstimmt, schaltet es den aktiven Leucht-Indikator ein oder aus.

---

### Beispiel 2: AudioVolumeControl-Service & mehrere Audio-Widgets

#### 1. Der Service (AudioVolumeControl, Singleton)

* **Zustand:** Verwaltet die asynchrone Verbindung zu **PipeWire** (oder PulseAudio).
* **Status-Broadcasts:** Bei jeder Änderung (durch externe Programme, Tasten am Monitor oder Touch-Eingaben) publiziert der Service auf `service.audio.state`:
  ```json
  Payload: {
    "volume": 0.65,
    "is_muted": false,
    "output_devices": [
      { "id": 1, "name": "iiyama HDMI", "is_default": true },
      { "id": 2, "name": "Headphones USB", "is_default": false }
    ]
  }
  ```

#### 2. Die Widgets (z. B. Slider im Sidepanel + Mute-Button im Top-Bar)

* **Abonnement:** Beide Widgets abonnieren beim Start das Topic `service.audio.state`.
* **Sync-Rendering:** Empfangen sie das Event, aktualisiert das Slider-Widget die Balkenposition auf `65%` und das Button-Widget schaltet das Icon auf "
  Lautsprecher an".
* **Gesten-Verarbeitung (Swipe up auf Slider):**
  Widget sendet Steuerungs-Befehl an den Broker:
  ```json
  Topic:   "service.audio.command"
  Payload: { "action": "SetVolume", "volume": 0.70 }
  ```
  Der Service empfängt dies, reguliert über PipeWire die System-Lautstärke und sendet das neue globale State-Event heraus, woraufhin sich alle Widgets
  gleichzeitig updaten.

---

## 5. Implementierungs-Fahrplan für den Core

Um diesen Service-Layer einzubauen, müssen folgende Schritte im Hauptprogramm durchgeführt werden:

### Schritt 1: API erweitern (`smearor-plugin-api`)

* Hinzufügen des Moduls `smearor-plugin-api/src/service.rs` (wie in Sektion 3 gezeigt).
* Exportieren der Typen im `lib.rs` des API-Crates.

### Schritt 2: Service-Manager im Core implementieren

* Erstellen einer neuen Komponente `smearor-swipe-launcher/src/service_manager.rs`.
* Analog zum `PluginManager` lädt der `ServiceManager` alle in der Konfigurationsdatei definierten System-Services dynamisch über FFI.
* Die geladenen Services werden in einer `DashMap<String, LoadedService>` verwaltet.

### Schritt 3: Event-Broker Routing anpassen (`smearor-swipe-launcher/src/messages.rs`)

Der Broker steuert nun auch die Services an:

1. Jede Nachricht mit dem Topic-Präfix `service.{service_id}/command` wird direkt an die `on_message`-Methode des entsprechenden Services in der
   `ServiceManager`-Map weitergeleitet.
2. Wenn ein Service eine Status-Nachricht an den Broker schickt, leitet dieser die Nachricht (z. B. via Broadcast-Verteilung) an alle passenden Widgets weiter.

---

## 6. Vorteile des Designs

* **Garantierte FFI-Stabilität:** Reine Datenkommunikation via JSON-Strings über Topics. Keine Abhängigkeit von spezifischen Structs oder C-Enums.
* **Extrem hohe UI-Performance:** Der rechenintensive Code (Pipewire-Abfragen, Signale, Systemüberwachung, `/proc`-Polling) läuft asynchron im
  Service-Hintergrund-Thread (Tokio). GTK-Widgets zeichnen ausschließlich Daten.
* **Geringer Speicher-Footprint:** Keine redundanten Verbindungen oder Threads für duplizierte Widgets.
* **Maximale Flexibilität:** Neue Widgets können mit minimalem Aufwand entwickelt werden, da sie lediglich JSON-Nachrichten senden und empfangen müssen, statt
  das gesamte Pipewire/D-Bus-Subsystem selbst zu implementieren.
