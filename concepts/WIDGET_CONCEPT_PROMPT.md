https://github.com/smearor/smearor-swipe-launcher

AGS ist Inspiration für den smearor swipe launcher . Hauptpunkt ist die Flexibilität.

Allerdings ist der Swipe Launcher insofern flexibel, dass er ein natives Plugin-System bereitstellt und sehr performant und sehr nativ ist.

Widgets sollen auch extern programmiert werden können und deshalb gibt es ein ABI stabiles plugin-api crate.

Man kann selbst Widgets implementieren.

Zweiter Punkt ist die native Touch-Bedienbarkeit.

Kannst du mir für den Smearor Swipe Launcher ein Konzept / Roadmap für die Implementierung von folgenden Widgets erstellen?

- App Launcher Widget Plugin: Konfigurierbar mit dem Pfad zu einer Desktop-Datei. Bei Klick wird das Programm gestartet. Die gestartete PID soll sich gemerkt
  werden. Bei Longpress soll die App anhand ihrer PID beendet werden.

- Audio Widget Plugin: Zeigt Lautstärke, Input/Output-Geräte und Mute-Status an. Mit Swipe Bewegungen auf dem Audio Widget soll die Lautstärke erhöht,
  verringert oder gemutet oder entmutet werden.

- Network Widget Plugin: WLAN-SSID, Verbindungsstatus und Internetanbindung.

- Bluetooth Widget Plugin: Geräte-Listen und Verbindungsstatus.

- Mpris Widget Plugin: Steuerelemente für Medienplayer (Play/Pause, Titel, Künstler, Album-Cover).

- Notifications Widget Plugin: Ein dediziertes System zur Verwaltung und Anzeige von Benachrichtigungen.