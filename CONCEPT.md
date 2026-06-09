# Konzept: `smearor-swipe-launcher`

**Touch-optimierter, rotierbarer und modularer Scrolling-App-Launcher für Wayland**

---

## 1. System-Architektur & Schichtenmodell

Das System basiert auf einer strikt entkoppelten, dreistufigen Architektur. Dies stellt sicher, dass der Kern des
Launchers stabil bleibt, während visuelle Komponenten und Logiken flexibel ausgetauscht oder erweitert werden können,
ohne die gesamte Anwendung neu kompilieren zu müssen.

* **Der Core (`smearor-swipe-launcher`):** Das Hauptprogramm verwaltet das Anwendungsfenster unter Wayland, steuert die
  Ausrichtung im Raum (Rotation), fängt globale Gesten ab und stellt das Layout-Gerüst bereit.
* **Die Schnittstelle (`smearor-plugin-api`):** Ein minimales, stabiles Verbindungselement, das den Vertrag zwischen dem
  Core und den Plugins definiert. Es regelt, wie Widgets an das Hauptfenster übergeben und wie Interaktionen (Klicks,
  Halten) an die Plugins zurückgemeldet werden.
* **Die Erweiterungen (Widget-Plugins):** Unabhängige, eigenständige Code-Pakete, die zu nativen Systembibliotheken (
  `.so`-Dateien) kompiliert werden. Jedes Plugin kapselt seine eigene Logik (z. B. Uhrzeit-Aktualisierung,
  DBus-Kommunikation für Musik oder Benachrichtigungen) und erzeugt ein spezifisches visuelles Element.

---

## 2. Das Kern-Fenster & UI-Layout

Das Hauptprogramm ist für die nahtlose Integration in die Wayland-Desktop-Umgebung verantwortlich und speziell für
physisch immersive Arbeitsumgebungen optimiert.

### Haupteinsatzzweck: Table-Top Smart-Desk (65" 4K-Touch)

Der primäre Einsatzzweck des Launchers ist die Bereitstellung von interaktiven Menubändern an allen vier Kanten eines
horizontalen Table-Top Smart-Desks mit einem **65" großen 4K-Touchscreen**. In dieser Umgebung sitzen Benutzer an
verschiedenen Seiten des Tisches zusammen und interagieren gemeinschaftlich mit dem System. Dies stellt besondere
Anforderungen an die Benutzeroberfläche:

* **Ausrichtungs-Präzision:** Die Menubänder müssen an jeder Kante (0°, 90°, 180°, 270°) perfekt gerendert und zur
  jeweiligen Tischseite hin ausgerichtet sein, sodass Benutzer die Elemente aus ihrer jeweiligen Sitzposition heraus
  aufrecht lesen können.
* **Großflächige Touch-Optimierung:** Aufgrund der enormen Bildschirmfläche von 65" sind alle Touch-Ziele besonders
  großzügig gestaltet (Fitts's Law). Icons, Schaltflächen und Abstände sind so dimensioniert, dass sie auch bei
  schnellen Gesten im Stehen oder Sitzen fehlerfrei getroffen werden.
* **4K-Skalierbarkeit:** Schriften und Vektorgrafiken sind vollkommen frei skalierbar und hochauflösend ausgelegt, um
  sowohl aus der Nähe als auch aus größerer Distanz gestochen scharf zu bleiben.

### Integration von `smearor-wrot-rotation`

Um Widgets an den Kanten des Smart-Desks flexibel rotieren zu können, integriert der Core das GTK 4-Widget
`RotationWidget` aus der externen Bibliothek **`smearor-wrot-rotation`**:

* **Visuelle Transformation:** Das `RotationWidget` nutzt performante GSK-Transforms (`GskTransform`), um das gesamte
  Rendering-Widget-Layout ohne Qualitätsverlust im Raum zu drehen (z. B. um 90° an der linken Kante oder 180° an der
  oberen Kante).
* **Eingabe-Transformation (Input/Output-Mapping):** Ein herkömmlicher Rotations-Transform in CSS oder GTK führt oft
  dazu, dass Touch- und Mauskoordinaten nicht mehr mit den visuellen Elementen übereinstimmen. `RotationWidget`
  transformiert alle Touch-, Maus- und Tastaturereignisse bidirektional. Dadurch verhalten sich Swipe- und Drag-Gesten
  auf gedrehten Widgets an jeder Kante exakt so, als ob sie in der Standardausrichtung bedient würden.

### Fenstermanagement & Positionierung

Der Launcher nutzt das Wayland Layer Shell Protokoll. Er besitzt keine Fensterdekorationen (Rahmen, Titelleisten) und
verhält sich wie ein systemeigenes Panel. Die Position auf dem Bildschirm ist direkt an den Rotationszustand des
Displays gekoppelt:

* **0° (Tischkante Unten):** Das Fenster verankert sich am unteren Rand und dehnt sich horizontal aus.
* **90° (Tischkante Links):** Das Fenster wandert an den linken Rand und dehnt sich vertikal aus.
* **180° (Tischkante Oben):** Das Fenster heftet sich an den oberen Rand und dehnt sich horizontal aus.
* **270° (Tischkante Rechts):** Das Fenster wandert an den rechten Rand und dehnt sich vertikal aus.

### Die UI-Hierarchie (Das "Band"-Prinzip)

Das Fenster ist intern in drei Bereiche unterteilt:

1. **Linker statischer Bereich:** Nimmt permanente Widgets auf, die beim Scrollen nicht herbeigeführt oder weggeschoben
   werden sollen (z. B. eine permanente Uhr oder Menü-Buttons).
2. **Zentrales Scroll-Band:** Ein hochperformanter, virtualisierter Container. Um auch bei Hunderten installierten Apps
   ein flüssiges Durchwischen (Swipen) mit konstanten 120 Hz zu garantieren, nutzt das Scroll-Band moderne,
   virtualisierte GTK 4-Listen-Widgets (`GtkListView` oder `GtkGridView`) in Kombination mit `GskTransform`. Anstatt für
   jeden Eintrag ein permanentes Widget im Speicher zu halten, werden nur die aktuell sichtbaren Widgets erzeugt und
   beim Scrollen dynamisch wiederverwendet (Widget-Recycling).
3. **Rechter statischer Bereich:** Analog zur linken Seite für permanente Elemente am anderen Ende des Launchers (z. B.
   ein Benachrichtigungs-Indikator).

---

## 3. Navigations- & Gesten-Logik

Die Interaktion mit dem Launcher folgt einem intuitiven, auf Touchscreens und Tastatur-Shortcuts optimierten Muster:

* **Horizontales Wischen (Links / Rechts):** Verschiebt den Inhalt des zentralen Scroll-Bands flüssig mit dem Finger
  oder der Maus.
* **Tastatur-Steuerung:** Eine globale Tastenkombination (z. B. `SUPER + PFEILTASTEN`) erlaubt es, das Band schrittweise
  per Code zu bewegen, falls kein Touchscreen genutzt wird.
* **Vertikales Wischen (Oben / Unten):** Ein Swipe nach oben öffnet ein übergeordnetes Menü (Parent Menu), während ein
  Swipe nach unten den Launcher minimiert oder schließt.

---

## 4. Das Plugin-System & Widget-Lebenszyklus

Die Erstellung der UI-Elemente erfolgt vollkommen dynamisch zur Laufzeit der Anwendung auf Basis einer hochperformanten,
robusten und sicheren Architektur.

### 4.1 Laden, Instanziieren & Zwei-Wege-Kommunikation

Beim Start liest der Core die Konfiguration ein. Für jeden Eintrag lädt der Launcher die entsprechende `.so`-Datei und
ruft eine standardisierte Konstruktor-Funktion des Plugins auf.

* **Zwei-Wege-Kommunikation (Event-Bus):** Um eine bidirektionale Interaktion zu ermöglichen, übergibt der Core beim
  Instanziieren ein sicheres Handle auf den Core-Kontext (`CoreContext` bzw. einen `EventChannel`). Über diesen
  Rückkanal kann das Plugin asynchron standardisierte Nachrichten an den Core senden, wie z. B.:
    * `RequestClose`: Den Launcher nach dem erfolgreichen Starten einer App schließen.
    * `TriggerParentMenu`: Ein übergeordnetes Menü öffnen oder das Band bewegen.
    * `EmitNotification`: Eine Systembenachrichtigung auslösen.
* **State-Separation (Virtualisierungs-Schnittstelle):** Um die UI-Virtualisierung des Scroll-Bands zu unterstützen,
  trennt die API strikt zwischen der reinen Daten-Struktur (State) des Plugins und der tatsächlichen UI-Generierung. Das
  Plugin liefert dem Core ein leichtgewichtiges Datenobjekt (Model), während das Binden an das reale, recycelte
  GTK-Widget erst bei Sichtbarkeit auf dem Bildschirm erfolgt.

### 4.2 Blockierungsfreies Threading-Modell

GTK 4 arbeitet strikt single-threaded im Haupt-Thread (UI-Thread). Jede blockierende Operation eines Plugins (z. B. das
Einlesen einer `.desktop`-Datei von einer langsamen Festplatte, das Warten auf eine DBus-Antwort via MPRIS oder
Netzwerk-Requests) würde die gesamte Benutzeroberfläche inklusive aller Animationen einfrieren.

* **Asynchrones Laden & Shimmer-Widgets:** Das Plugin-Trait verbietet zeitintensive Berechnungen im Initialisierungs-
  und Widget-Erstellungs-Schritt. Plugins bauen stattdessen sofort ein leeres Skelett-Widget (
  Shimmer/Placeholder/Skeleton) auf.
* **Background Tasks:** Schwere I/O- oder Rechenaufgaben werden in asynchrone Tasks ausgelagert (z. B. via Tokio-Runtime
  oder über einen `glib::MainContext::channel`).
* **UI-Updates im GTK-Haupt-Thread:** Sobald die Daten bereitstehen, sendet der asynchrone Task eine Nachricht an den
  GTK-Haupt-Thread zurück, der das Skelett-Widget sicher und ohne Blockaden mit den echten Daten befüllt.

### 4.3 Interaktions-Pipeline

Die Erkennung von Berührungen übernimmt zentral der Core-Launcher, um ein einheitliches Systemverhalten zu garantieren:

* **Primäre Aktion (Kurzer Tipp / Klick):** Der Core erkennt das Loslassen des Fingers und signalisiert dem Plugin, die
  Hauptfunktion auszuführen (z. B. App starten oder Musik pausieren). Hierbei wird dem Plugin die aktuelle Rotation
  mitgeteilt, sodass gestartete Apps mit dem entsprechenden Parameter aufgerufen werden können.
* **Sekundäre Aktion (Langer Druck / Longpress):** Der Core registriert das Halten eines Elements und weist das Plugin
  an, die Sekundäroperation auszuführen (z. B. Kontextmenü oder Einstellungsdialog öffnen).

### 4.4 Speicherintegrität an der FFI-Grenze

Die Übergabe von Rust-Trait-Objekten über dynamische Bibliotheksgrenzen (`libloading`) birgt erhebliche Risiken für
Speicherfehler (Undefined Behavior), wenn herkömmliche `Box::into_raw` und `Box::from_raw` Muster verwendet werden:

1. **Der Allokator-Konflikt:** Wenn der Core-Launcher und das Plugin mit unterschiedlichen Compilerversionen,
   Optimierungsstufen oder Allokatoren (z. B. `jemalloc` im einen, System-Allokator im anderen) kompiliert wurden, führt
   das Freigeben des Speichers im Hauptprogramm zu einem sofortigen Crash.
2. **Fehlende Destruktor-Kontrolle:** Der Core darf den Speicher eines Plugins nicht willkürlich in seinem eigenen
   Kontext freigeben.

* **Lösung (Plugin-gesteuertes Freigeben):** Jedes Plugin exportiert zusätzlich zu seinem Konstruktor eine dedizierte
  FFI-Funktion:
  ```rust
  unsafe extern "C" fn _smearor_plugin_destroy(ptr: *mut dyn MenuEntry)
  ```
  Wenn der Launcher ein Plugin entlädt oder schließt, übergibt er den Zeiger zurück an diese Funktion des Plugins.
  Dadurch wird sichergestellt, dass das Plugin seine eigenen Ressourcen mit exakt demselben Compiler und Allokator
  freigibt, mit dem sie erzeugt wurden. Alternativ wird das Speicher-Handling in ein C-kompatibles `ffi::CustomDeleter`
  -Muster ausgelagert.

---

## 5. Konfigurations-Design

Die gesamte Oberfläche wird über eine einzige, zentrale Textdatei (z. B. im TOML-Format) konfiguriert. Diese Datei ist
dreigeteilt und spiegelt das UI-Layout wider:

* Globale Einstellungen (Animationsgeschwindigkeiten, Standardrotation).
* Listen für die jeweiligen Positionen (Links persistent, Mitte scrollbar, Rechts persistent).

Jeder Eintrag definiert lediglich den Systempfad zur gewünschten Plugin-Bibliothek sowie einen freien Variablen-Block.
Der Core-Launcher muss die Logik der Widgets nicht verstehen; er reicht die spezifischen Parameter einfach blind an das
jeweilige Plugin weiter. Dadurch kann ein Benutzer neue Widgets hinzufügen oder Parameter ändern, ohne dass der
Launcher-Quellcode modifiziert werden muss.