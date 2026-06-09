# `smearor-swipe-launcher`: Swipe-gesteuerter Scrolling-App-Launcher

## Vision

Ein kleines Band am unteren Rand des Bildschirms, das man mit den Fingern nach links und rechts swipen kann um eine

längere Liste von Apps zu durchlaufen. Mit einem Klick auf eine App wird diese gestartet. Unterstützt Rotation.

## Fenster

- Rust

- GTK-4 Fenster

    - Layer-Shell-Layer

    - Keine Decorations

    - Rotation bestimmt auch die Position des Layers:

        - 0 Degrees: unten

        - 90 Degrees: links

        - 180 Degrees: oben

        - 270 Degrees: rechts

    - Child: RotationWidget

        - Child: SwipeWidget

            - Swipe Left (Touch or Mouse or SUPER+ARROW-KEY): Scroll left

            - Swipe Right (Touch or Mouse or SUPER+ARROW-KEY): Scroll right

            - Swipe Up (Touch or Mouse or SUPER+ARROW-KEY): Parent Menu

            - Swipe Down (Touch or Mouse or SUPER+ARROW-KEY): Scroll

            - Child: MenuEntryWidget (Persistent, links von scrollable)

            - Child: MenuEntryWidget (Dynamic)

                - Shows the desktop entry icon

                - Shows the desktop entry name

                - Shows the desktop entry description

                - Click or Touch: Execute Primary Action

                    - Launch menu entry (if smearor-wrot: with --rotation parameter)

                - Longpress or Longtouch: Execute Secondary Action

            - Child: MenuEntryWidget (Persistent, links von scrollable)

## MenuEntryWidget

Prinzipiell rendert das MenuEntryWidget ein einzelnes GTK Widget.

Herausforderung:

- Widget: Launch-Desktop-Entry

- Widget: Date- / Time

- Widget: Notification

- Widget: MPRIS

Diese Widgets sollen nicht fest einprogrammiert werden, sondern anhand von einem Konfigurationsfile eingelesen werden.

Das ist ein fantastisches und extrem gut durchdachtes Konzept! Ein touch-optimierter, rotierbarer Launcher mit flüssigem
Scrolling schließt eine echte Lücke im Wayland-Ökosystem (besonders für Linux-Tablets oder Convertibles).

Da du die Widgets vollständig dynamisch per Konfigurationsdatei laden möchtest, reicht ein einfacher match auf
vordefinierte Enums nicht aus. Du brauchst ein Plugin- oder Komponenten-System, bei dem jede Widget-Art eine eigene
Logik kapselt.

Hier ist der Architektur-Plan und die konkrete Umsetzung, wie du dieses System in Rust und GTK 4 aufbaust.


