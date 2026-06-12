# CONCEPT Audio Widget Plugin

Hier ist die konkrete Ausarbeitung für das **Audio Widget Plugin** des *Smearor Swipe Launchers*. Da das System auf Ubuntu/Linux läuft, wird hierbei voll auf
eine native Steuerung über **PipeWire** (via `pw-loop` oder `pulsectil-rs` / `libpulse-binding` für die PulseAudio-Kompatibilitätsschicht) gesetzt.

Das Konzept trennt die UI-Interaktionen (Touch & Maus) sauber von der Systemlogik und nutzt das ABI-stabile Plugin-Crate.

---

### 1. Architektur und Datenstrukturen

Für das Widget wird ein interner Zustand definiert, der sowohl den aktuellen Systemstatus cached als auch die Geometrie für die Interaktionsberechnung (
Gesten/Maus) hält.

```rust
// Im ABI-stabilen Plugin-Crate definiert
pub enum AudioInputEvent {
    TouchSwipe { delta_y: f32 },
    TouchTap,
    MouseScroll { delta_y: f32 },
    MouseMiddleClick,
}

pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub is_default: bool,
}

pub struct AudioWidgetState {
    pub volume: f32,          // 0.0 bis 1.0 (oder >1.0 für Overdrive)
    pub is_muted: bool,
    pub output_devices: Vec<AudioDevice>,
    pub input_devices: Vec<AudioDevice>,
    pub show_device_list: bool,
}

```

---

### 2. Interaktions-Logik (Touch & Maus)

Das Widget implementiert die Event-Verarbeitung für beide Eingabemethoden über dieselbe Engine, um Code-Duplizierung zu vermeiden.

#### Event-Mapping Tabelle

| Aktion                    | Eingabe (Touch)          | Eingabe (Maus)          | Interne Funktion       |
|---------------------------|--------------------------|-------------------------|------------------------|
| **Lautstärke erhöhen**    | Swipe nach oben / rechts | Mausrad hoch            | `change_volume(0.05)`  |
| **Lautstärke verringern** | Swipe nach unten / links | Mausrad runter          | `change_volume(-0.05)` |
| **Mute / Unmute**         | Kurzer Tap (Zentrum)     | Mittelklick (Scrollrad) | `toggle_mute()`        |
| **Gerätewechsel**         | Longpress (2 Sek.)       | Rechtsklick             | `toggle_device_list()` |

#### Implementierung der Event-Verarbeitung (Pseudocode)

```rust
impl SmearorWidget for AudioWidget {
    fn handle_input(&mut self, event: AudioInputEvent) {
        match event {
            AudioInputEvent::TouchSwipe { delta_y } => {
                // Konvertiere die Pixel-Bewegung in Prozentänderung
                let vol_change = delta_y * 0.005;
                self.apply_volume_change(vol_change);
            }
            AudioInputEvent::MouseScroll { delta_y } => {
                // Mausrad liefert meist diskrete Werte (-1.0 oder 1.0)
                let vol_change = delta_y * 0.05;
                self.apply_volume_change(vol_change);
            }
            AudioInputEvent::TouchTap | AudioInputEvent::MouseMiddleClick => {
                self.toggle_mute();
            }
        }
    }
}

```

---

### 3. Linux-Systemanbindung (PipeWire / PulseAudio)

Die eigentliche Kommunikation mit dem Audio-Server läuft asynchron im Hintergrund, um das GUI-Rendering (60+ FPS) nicht durch I/O-Blockaden zu bremsen.

1. **Initialisierung:** Das Plugin abonniert beim Start den Event-Stream des Audio-Servers (`pa_context_subscribe`).
2. **Sinken & Quellen auslesen:** Abfrage der Standard-Ausgabe (Default Sink, z. B. `@DEFAULT_SINK@`).
3. **Ausführung der Änderungen:**

* **Lautstärke:** `pactl set-sink-volume @DEFAULT_SINK@ +5%` (bzw. die entsprechende FFI-Funktion).
* **Mute:** `pactl set-sink-mute @DEFAULT_SINK@ toggle`.

---

### 4. UI- und Layout-Konzept

Das visuelle Feedback muss sowohl für Touch (große Trefferflächen) als auch für die Maus präzise sein.

* **Zentraler Fortschrittsbalken (Slider):** Füllt den Hintergrund des Widgets proportional zur Lautstärke (0–100%).
* *Farbschema:* Ein dezentes Grau im Hintergrund, ein sattes Blau/Cyan für die aktuelle Lautstärke. Bei `Mute` wechselt der Balken zu einem gedimmten Rot/Grau.


* **Icon-Indikator:** Ein dynamisches Lautsprechersymbol im Zentrum:
* 🔇 Muted
* 🔈 Leise (1–33%)
* 🔉 Mittel (34–66%)
* 🔊 Laut (67–100%)


* **Text-Overlay:** Eine kleine, cleane Textzeile am oberen Rand zeigt den Namen des aktuellen Ausgabegeräts an (z. B. *„iiyama GB3261U HDMI“* oder
  *„Kopfhörer“*).

---

### 5. Technische Hürden & Optimierungen

* **Scroll-Event-Debouncing:** Mausräder können sehr schnell gedreht werden. Das Widget sollte die Events innerhalb eines Frames (ca. 16ms) akkumulieren und
  gebündelt an PipeWire senden, um den D-Bus / Audio-Socket nicht zu fluten.
* **Overscroll / Overdrive:** Unter Linux ist es oft möglich, die Lautstärke über 100% (bis zu 150%) zu heben. Das Widget sollte in der Konfiguration eine
  Option namens `allow_overdrive: bool` bereitstellen. Wenn aktiv, färbt sich der Slider ab 100% orange.