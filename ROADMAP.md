# Entwicklungsplan: `smearor-swipe-launcher`

## Phase 1: Das Fundament & Die Schnittstelle (Minimum Viable Product)

*Fokus: Architektur aufsetzen, ein Fenster anzeigen und das erste Plugin zur Laufzeit laden.*

### 🏁 Milestone 1.1: Die API & Das Test-Plugin

* [x] **Projekt-Workspace einrichten:** Erstellung der Rust-Workspace-Struktur mit dem Core-Launcher, der API-Library
  und einem Verzeichnis für Plugins.
* [x] **Definition der Plugin-API (State-Separation):** Entwurf des Kern-Traits mit einer strikten Trennung zwischen
  Daten-Modell (State) und der UI-Generierung (Vorbereitung für Virtualisierung), inklusive Übergabe eines
  `CoreContext` / `EventChannel` an den Konstruktor.
* [x] **C-ABI Speicherabsicherung an der FFI-Grenze:** Integration eines sicheren Allokations- und Destruktionsmusters (
  z. B. FFI-Export von `_smearor_plugin_destroy`) in die API-Definition, damit Plugins ihren eigenen Speicher sauber
  freigeben können.
* [x] **Das erste funktionale Plugin:** Entwicklung eines minimalen Uhrzeit- oder Text-Widgets als dynamische
  Bibliothek (`.so`), das die Schnittstelle implementiert.

### 🏁 Milestone 1.2: Der Plugin-Loader im Core

* [x] **Dynamisches Laden (`libloading`):** Implementierung der Logik im Core-Launcher, die eine `.so`-Datei zur
  Laufzeit öffnet, den Konstruktor anspringt, dem Plugin den `CoreContext` übergibt und das Plugin sicher im Speicher
  hält.
* [x] **Speicher-Absicherung & Allokator-Sicherheit:** Implementierung eines Registrierungs- und Cleanup-Systems im
  Core, das beim Schließen eines Plugins den Zeiger an die plugin-interne FFI-Destruktor-Funktion übergibt, um
  Allokator-Konflikte zu vermeiden.
* [x] **Erstes visuelles Lebenszeichen:** Ein einfaches GTK 4-Fenster öffnet sich erfolgreich und zeigt Plugin-Widgets
  an. Plugins liefern Daten (State) über FFI, Core erstellt UI basierend auf diesen Daten (State-Separation
  implementiert).

---

## Phase 2: Wayland-Integration & Das "Band"-Layout

*Fokus: Das Fenster an das System anpassen und die touch-optimierte Navigation aufbauen.*

### 🏁 Milestone 2.1: Das Scrollbare Band & Gesten-Erkennung

* [x] **Layout-Dreiteilung:** Aufbau der UI-Struktur im Hauptfenster (Linker statischer Bereich, zentraler
  Scroll-Container, rechter statischer Bereich).
* [x] **Zentralisiertes Gesten-Handling:** Der Core fängt Klicks (Taps) und langes Drücken (Longpress) auf den
  dynamischen Widgets ab und leitet sie korrekt an die primären und sekundären Funktionen des jeweiligen Plugins weiter.

---

## Phase 3: Dynamische Konfiguration & Rotation

*Fokus: Den Launcher über eine Datei steuerbar machen und auf Hardware-Änderungen reagieren.*

### 🏁 Milestone 3.1: TOML-Parser & JSON-Brücke

* [x] **Konfigurations-Engine:** Implementierung eines Parsers (via Serde), der die zentrale `config.toml` einliest.
* [x] **Dynamischer UI-Aufbau:** Der Core durchläuft die Listen für links, rechts und das Scroll-Band, lädt die
  definierten Plugins und fügt sie an den richtigen Stellen im Layout ein.
* [x] **Parameter-Übergabe:** Der Core extrahiert die spezifischen Einstellungsblöcke aus der TOML-Datei und übergibt
  sie beim Laden als JSON-Schnittstelle an das jeweilige Plugin.

### 🏁 Milestone 3.2: Rotations- & Positionssynchronisation

* [x] **Integration von `smearor-wrot-rotation`:** Einbindung des `RotationWidget` aus der externen Bibliothek, um die
  Widgets an den jeweiligen Tischkanten (90°, 180°, 270°) visuell und interaktiv (Input/Output-Koordinaten-Mapping)
  sauber zu rotieren.
* [x] **Rotations-Parameter-Weitergabe:** Erweiterung der Primäraktion, sodass die aktuelle Display-Orientierung
  ermittelt und beim Ausführen an das Plugin übergeben wird (Vorbereitung für den `--rotation` Parameter beim
  App-Start).

---

## Phase 4: Core-Widgets (Die App-Ökosystem-Basis)

*Fokus: Entwicklung der primären Plugins, die den Launcher nützlich machen.*

### 🏁 Milestone 4.1: Das App-Launcher-Plugin

* [ ] **Blockierungsfreier Desktop-Entry-Parser:** Entwicklung eines asynchronen Plugins, das System-`.desktop`-Dateien
  im Hintergrund (Tokio / Glib-Channel) einliest. Währenddessen zeigt die UI ein Shimmer/Placeholder-Skelett an, das
  nach dem Laden blockierungsfrei aktualisiert wird.
* [ ] **Zwei-Wege App-Ausführung:** Implementierung der Start-Logik. Bei Klick wird die App gestartet – unter
  Berücksichtigung des Rotations-Parameters. Nach erfolgreichem Start sendet das Plugin ein `RequestClose` Signal über
  den Event-Kanal an den Core, um den Launcher zu schließen.

### 🏁 Milestone 4.2: Das MPRIS-Medien-Plugin

* [ ] **Asynchrones MPRIS-Plugin:** Entwicklung eines Medien-Widgets, das sich asynchron via DBus an aktive Player
  hängt. Die DBus-Kommunikation läuft in einem Hintergrund-Task und blockiert niemals den GTK-UI-Thread (keine Ruckler
  beim Wischen).
* [ ] **Touch-Mediensteuerung:** Klares Widget-Layout mit Album-Art-Anzeige und großflächigen Touch-Schaltflächen für
  Wiedergabe/Pause/Überspringen, optimiert für Tischkanten-Interaktion.

### 🏁 Milestone 4.3: Das Uhrzeit- & Kalender-Widget

* [ ] **Präzises Uhrzeit-Widget:** Einbindung eines hochperformanten, blockierungsfreien Zeitanzeige-Widgets mit
  konfigurierbaren Layouts (analog/digital) und Zeitzonen.
* [ ] **Skelettierte Kalender-Übersicht:** Touch-Interaktion auf der Uhr öffnet eine kleine, flüssig rotierte
  Kalender-Übersicht mit anstehenden Kalenderereignissen, asynchron geladen im Hintergrund.
* [ ] **Tisch-Skalierung:** Optimierung der Schriftgröße und des Kontrasts, um die Zeit auf dem 65" Tisch aus jeder
  Richtung lesbar zu machen.

### 🏁 Milestone 4.4: Das Benachrichtigungs-Widget (Notification Widget)

* [ ] **DBus Notification Daemon Listener:** Integration eines asynchronen Empfängers für den Standard
  `org.freedesktop.Notifications`-DBus-Dienst im Plugin.
* [ ] **Notification-Banner & Badge:** Darstellung eines Benachrichtigungs-Zählers im permanenten Bereich und flüssige
  Einblendung von neuen Bannern am Menuband.
* [ ] **Touch-Gesten-Interaktion:** Wischgesten (Swipe-to-dismiss) zum Schließen von Benachrichtigungen, die speziell
  für den großen Touchscreen entwickelt wurden.

### 🏁 Milestone 4.5: Layer-Shell & Positionierung

* [ ] **Layer-Shell-Integration:** Einbindung des Wayland-Protokolls, um die Fensterdekorationen komplett zu entfernen
  und den Launcher als System-Panel zu definieren.
* [ ] **Exklusive Zonen:** Konfiguration des Fensters, sodass es standardmäßig am unteren Bildschirmrand fixiert ist und
  andere geöffnete Anwendungsfenster bei Bedarf beiseite schiebt.
* [ ] **Dynamische Layer-Anpassung:** Implementierung einer Logik, die das Fenster bei einer Rotationsänderung zur
  Laufzeit an den korrekten Bildschirmrand verschiebt (0° unten, 90° links, etc.) und das Layout von horizontal auf
  vertikal spiegelt.

### 🏁 Milestone 4.6: Virtualisierung des Scroll-Bands (Performance)

* [ ] **GtkListView/GridView Integration:** Verwendung von modernen GTK 4-Listen-Widgets (`GtkListView`/`GtkGridView`)
  in Kombination mit `GskTransform` im zentralen Scroll-Bereich, um ein hocheffizientes Widget-Recycling bei großen
  Datenmengen (z. B. 200+ Apps) zu realisieren.

### 🏁 Milestone 4.7: Touch-Optimierung für 65" 4K Smart-Desks

* [ ] **Touch-Optimierung für 65" 4K Smart-Desks:** Anpassung aller Widget-Abmessungen, Icons und Abstände an das
  65-Zoll-Touch-Erlebnis (Fitts's Law) und Sicherstellung gestochen scharfer Skalierbarkeit.

---

## Phase 5: Polishing, Performance & Feinschliff

*Fokus: Gesten verfeinern, Performance optimieren und das System alltagstauglich machen.*

### 🏁 Milestone 5.1: Erweiterte Gesten & Shortcuts

* [ ] **Vertikale Swipes:** Implementierung der Erkennung für Wischgesten nach oben (Parent Menu aufrufen) und nach
  unten (Launcher minimieren).
* [ ] **Tastatur-Navigation:** Registrierung globaler Shortcuts (`SUPER + PFEILTASTEN`), um das Band auch ohne
  Touchscreen präzise steuern zu können.

### 🏁 Milestone 5.2: CSS-Styling & Robustheit

* [ ] **Hot-Reloading CSS:** Integration einer GTK-CSS-Struktur, die es erlaubt, das Aussehen des Launchers (Farben,
  Abstände, Rundungen) über eine externe Stylesheet-Datei anzupassen, idealerweise mit Live-Aktualisierung bei
  Änderungen.
* [ ] **Leistungs-Feinschliff der Virtualisierung:** Profiling und Optimierung der Swipe-Animationen bei 120 Hz unter
  maximaler Last mit Hunderten von virtualisierten Listeneinträgen.
* [ ] **Fehler-Kapselung (Panic-Handling):** Absicherung des Core-Launchers gegen fehlerhafte Drittanbieter-Plugins.
  Wenn ein Widget im laufenden Betrieb abstürzt, fängt der Core dies ab, blendet das Widget aus und verhindert den
  Absturz des gesamten Launchers.