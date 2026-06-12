# MPRIS Widget Concept

Ein MPRIS-Widget (Media Player Remote Interfacing Specification) ist die Brücke zwischen deinem Desktop und der Audiowiedergabe. Da du Linux mit GNOME und
Hyprland nutzt, interagiert dieses Widget direkt mit dem D-Bus-Standard, den fast alle Linux-Player (Spotify, VLC, Firefox, Audacious, etc.) implementieren.

Hier ist der Funktionsumfang, den ein "umfassendes" MPRIS-Widget abdecken sollte:

### 1. Kern-Steuerung (Die Basics)

Dies sind die Befehle, die das Widget an den Player sendet:

* **Wiedergabekontrolle:** Play, Pause, Play/Pause-Toggle, Stop.
* **Navigation:** Vorheriger Titel, Nächster Titel.
* **Seek-Funktion:** Vor- und Zurückspulen (z.B. +/- 10 Sekunden).
* **Loop-Mode:** Umschalten zwischen "kein Loop", "Loop Titel", "Loop Playlist".
* **Shuffle:** Ein-/Ausschalten der Zufallswiedergabe.

### 2. Metadaten-Anzeige (Visualisierung)

Das Widget sollte die Informationen über den aktuellen Titel dynamisch beziehen:

* **Titel & Interpret:** Anzeige des Songtitels und des Künstlers.
* **Album-Art:** Einbindung des Cover-Bildes (wird oft als URL oder lokaler Pfad über D-Bus gesendet).
* **Fortschrittsbalken:** Anzeige der aktuellen Position im Lied im Verhältnis zur Gesamtlänge (mit interaktivem Slider zum Springen).
* **Status-Indikator:** Dynamisches Icon, das den Status anzeigt (z. B. ein pausiertes Icon, wenn der Player pausiert ist).

### 3. Fortgeschrittene Features (Der "Pro"-Bereich)

Ein exzellentes Widget geht über die bloße Anzeige hinaus:

* **Player-Auswahl (Multiplexing):** Wenn mehrere Player laufen (z. B. Spotify im Hintergrund und Firefox für ein Video), sollte das Widget ein Dropdown oder
  einen Umschalter bieten, um den Ziel-Player zu wählen.
* **Volume-Integration:** Synchronisation mit der individuellen Lautstärke des Players (nicht nur der System-Master-Lautstärke).
* **D-Bus Signal-Handling:** Das Widget sollte bei jeder Statusänderung (z. B. ein neuer Song startet automatisch) durch D-Bus-Events "aufwachen" und sich
  aktualisieren, anstatt die Daten ständig abzufragen (Polling-Vermeidung).
* **Window-Integration:** Optionales Öffnen des Fensters des Players per Klick (falls unterstützt).

---

### Technische Struktur eines MPRIS-Widgets

Um dies in Rust/AGS umzusetzen, benötigst du eine saubere Trennung von der D-Bus-Kommunikation.

#### Struktur-Vorschlag für die Konfiguration:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct MprisSettings {
    pub show_cover_art: bool,
    pub show_progress_bar: bool,
    pub player_filter: Vec<String>, // Liste erlaubter Player (z.B. ["spotify", "firefox"])
    pub update_interval_ms: u64,
}

impl Default for MprisSettings {
    fn default() -> Self {
        Self {
            show_cover_art: true,
            show_progress_bar: true,
            player_filter: vec![],
            update_interval_ms: 1000,
        }
    }
}

```

### Zusammenfassung der Architektur-Tipps:

* **Zustand:** Nutze den D-Bus (`zbus` oder `mpris` crate in Rust), um dich an das `org.mpris.MediaPlayer2` Interface zu hängen.
* **Latenz:** Da MPRIS-Signale asynchron sind, sollte dein Widget bei einer `PropertiesChanged`-Nachricht sofort reagieren.
* **Robustheit:** Behandle den Fall, dass kein Player läuft (das Widget sollte sich in diesem Fall ausblenden oder einen "Kein Medium" Status anzeigen).

Möchtest du, dass wir für dieses MPRIS-Widget eine Trait-Definition entwerfen, die es erlaubt, verschiedene Player-Backends (Spotify vs. VLC) einheitlich
anzusprechen?