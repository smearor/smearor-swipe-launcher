# Notifications Widget Concept

Ein Notifications-Widget unter Linux (besonders in einer Hyprland-Umgebung) ist eines der kritischsten Elemente, da es die gesamte systemweite Kommunikation
bündelt. Da es bei dir mit AGS und Rust zusammenarbeitet, sollte der Funktionsumfang so gestaltet sein, dass er den **Desktop-Notification-Standard (Desktop
Notifications Specification)** vollständig ausschöpft.

Hier ist der Funktionsumfang, unterteilt in die wichtigsten Ebenen:

### 1. Kern-Funktionen (Standard)

* **Real-time Empfang:** Anzeige eingehender Benachrichtigungen via D-Bus (`org.freedesktop.Notifications`).
* **Kategorisierung (Urgency):** Unterscheidung zwischen `Low`, `Normal` und `Critical`. Das Widget sollte diese optisch unterschiedlich behandeln (z.B.
  farbliche Akzente oder Animationen für kritische Nachrichten).
* **Aktions-Buttons:** Viele Benachrichtigungen (z. B. von E-Mail-Clients oder Messengern) bieten Aktionen an (z. B. "Antworten", "Löschen", "Archivieren").
  Dein Widget muss diese Buttons rendern und die Rückmeldung korrekt an den Absender-Prozess übermitteln.

### 2. Management & Historie

* **Benachrichtigungshistorie:** Ein erweiterbares Panel, das bereits gelesene Benachrichtigungen speichert, falls der Benutzer sie kurzzeitig verpasst hat.
* **Filterung & Stummschaltung (Do Not Disturb):** Ein Modus, der Benachrichtigungen unterdrückt oder nur in der Historie ablegt, ohne den Benutzer durch
  Pop-ups zu stören.
* **Gruppierung:** Wenn ein Programm (z.B. ein Chat-Client) mehrere Nachrichten in kurzer Zeit sendet, sollte das Widget diese gruppieren (z.B. "3 neue
  Nachrichten von X"), statt den Desktop mit einzelnen Pop-ups zu fluten.

### 3. Visuelle & Interaktive Features

* **Custom Widgets innerhalb der Benachrichtigung:** Fortschrittsbalken (z. B. für Datei-Downloads oder Audio-Uploads).
* **Timeout-Steuerung:** Die Möglichkeit, ein "Sticky"-Flag zu setzen, damit kritische Benachrichtigungen erst verschwinden, wenn der Benutzer sie explizit
  wegklickt.
* **Animationen:** Sanftes Ein- und Ausblenden (`Fade-in`/`Slide-in`), was in Hyprland mit AGS extrem flüssig umgesetzt werden kann.

### 4. Technische Struktur (Die "Rust-AGS" Verbindung)

Da du Rust als Backend für deine AGS-Logik nutzt, wäre hier eine effiziente Architektur sinnvoll:

#### Empfohlene Konfigurations-Struktur

Damit das Widget für deine Bedürfnisse flexibel bleibt:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct NotificationSettings {
    pub max_visible: u32,
    pub timeout_ms: u64,
    pub show_history: bool,
    pub critical_only_do_not_disturb: bool,
    pub ignore_apps: Vec<String>, // Liste von Apps, die niemals benachrichtigen dürfen
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            max_visible: 5,
            timeout_ms: 5000,
            show_history: true,
            critical_only_do_not_disturb: true,
            ignore_apps: vec![],
        }
    }
}

```

### Zusammenfassung der Architektur-Strategie

* **Backend:** Nutze in Rust ein Crate wie `notify-rust` oder `zbus`, um den D-Bus-Server für Benachrichtigungen zu implementieren.
* **Frontend:** Dein AGS-Widget fungiert nur als "View". Es erhält die Daten vom Rust-Backend und rendert sie.
* **Interaktion:** Der wichtigste Punkt bei Notifications ist das **`Dismiss` und `Invoke**`. Dein Widget muss sicherstellen, dass beim Klick auf eine Nachricht
  das entsprechende Signal zurück an den Sender-Prozess gesendet wird, damit z.B. eine E-Mail als gelesen markiert wird.

Möchtest du, dass wir für das Notifications-Backend eine kleine Schnittstelle in Rust entwerfen, damit du die eingehenden Benachrichtigungen filtern oder loggen
kannst, bevor sie im AGS-Widget landen?