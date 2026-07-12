# Clock Widget Concept

Ein "Clock-Widget", das sowohl flexibel als auch umfangreich sein soll, muss weit über das einfache Anzeigen der Uhrzeit hinausgehen. Wenn du es für eine
Umgebung wie Hyprland (in Rust/AGS) entwickelst, solltest du es modular aufbauen.

Hier sind die Features, die ein modernes, professionelles Clock-Widget ausmachen:

### 1. Zeitanzeige & Formatierung (Kern-Funktionen)

* **Flexible Formatierung:** Benutzer sollten zwischen 12h/24h-Modus, verschiedenen Datumsformaten (ISO, lokalisiert) und der Anzeige von Sekunden oder
  Millisekunden umschalten können.
* **Weltzeit-Support:** Unterstützung für mehrere Zeitzonen gleichzeitig (z. B. "Local", "Work", "Family").
* **Kalender-Integration:** Ein Klick auf die Uhr sollte ein Dropdown- oder Overlay-Widget mit einem Kalender öffnen, der Termine (z. B. aus Google Calendar
  oder `.ics`-Dateien) anzeigt.

### 2. Visuelle Anpassung (Flexibilität)

* **Dynamische Themen:** Das Widget sollte auf das Theme von Hyprland reagieren (Dark/Light-Mode oder sogar "Material You"-ähnliche Farbpaletten, die auf dem
  Wallpaper basieren).
* **Layout-Varianten:**
* *Minimal:* Nur Uhrzeit.
* *Erweitert:* Uhrzeit, Datum und Wochentag.
* *Full:* Zusätzlich mit nächstem Termin oder Timer-Status.


* **Custom Tooltips:** Beim Hovern sollten detailliertere Informationen erscheinen (z. B. Kalenderwoche, Mondphase, Jahresfortschritt in Prozent).

### 3. Kontextbezogene Funktionen (Umfang)

* **Timer & Stoppuhr:** Ein integrierter Timer, der beim Ablauf eine Systembenachrichtigung (via `libnotify` oder D-Bus) auslöst.
* **Fokus-Timer (Pomodoro):** Ein dedizierter Modus für Produktivität, der die verbleibende Zeit deines Arbeitsintervalls in der Uhr-Sektion visualisiert.
* **Alarm-Management:** Anzeige des nächsten aktiven Alarms und die Möglichkeit, diesen direkt aus dem Widget zu deaktivieren.

### 4. System-Integration

* **D-Bus Interaktion:** Die Möglichkeit, die Uhr per D-Bus zu steuern oder Daten von anderen Systemdiensten abzurufen.
* **Event-Handling:** Integration in einen Notification-Daemon. Wenn ein Alarm abläuft, sollte sich das Design der Uhr (z. B. durch eine Puls-Animation oder
  Farbumschlag) ändern, um den Fokus des Benutzers zu gewinnen.
* **Drag-and-Drop / Modulares System:** Wenn du AGS nutzt, solltest du die Uhr so bauen, dass sie in einer `Box` in deiner Bar oder als "Floating-Window" (
  Overlay) gleichermaßen funktioniert.

---

### Vorschlag zur technischen Struktur (Datenmodell)

Um die Flexibilität zu garantieren, solltest du die Logik von der Anzeige trennen. Ein "Clock-Service" in deinem Rust-Code könnte die Zeitdaten liefern, und das
Widget konsumiert diese.

### Zusammenfassung der Architektur-Strategie

Wenn du das Widget in Rust für AGS umsetzt, empfehle ich dieses Schema:

1. **`ClockService`:** Ein Struct, das in einem `tokio::spawn` oder einer Schleife tickt und die Zeitdaten aktuell hält.
2. **`Config` Struct:** Hier nutzt du das `serde`-Pattern aus deiner vorherigen Frage, um dem Benutzer die Wahl zwischen verschiedenen Layouts, Formaten und
   Zeitzonen über eine `toml`-Datei zu geben.
3. **Visualisierung:** Nutze die `Signal`-Struktur von AGS, damit sich das Widget bei jeder Änderung der Zeitdaten reaktiv aktualisiert, ohne den Rest der Shell
   neu zu rendern.

Möchtest du, dass wir für dieses Clock-Widget ein konkretes `Settings`-Struct entwerfen, das alle diese Konfigurationsmöglichkeiten per TOML abdeckt?