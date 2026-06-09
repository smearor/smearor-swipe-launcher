# `smearor-swipe-launcher`: Swipe-Driven Scrolling App Launcher

## Vision

A small ribbon at the bottom of the screen that you can swipe left and right with your fingers to navigate a

longer list of apps. Clicking on an app launches it. Supports rotation.

## Window

- Rust

- GTK-4 Window

    - Layer-Shell-Layer

    - No Decorations

    - Rotation also determines the position of the layer:

        - 0 Degrees: bottom

        - 90 Degrees: left

        - 180 Degrees: top

        - 270 Degrees: right

        - Child: RotationWidget

            - Child: SwipeWidget

                - Swipe Left (Touch or Mouse or SUPER+ARROW-KEY): Scroll left

                - Swipe Right (Touch or Mouse or SUPER+ARROW-KEY): Scroll right

                - Swipe Up (Touch or Mouse or SUPER+ARROW-KEY): Parent Menu

                - Swipe Down (Touch or Mouse or SUPER+ARROW-KEY): Scroll

                - Child: MenuEntryWidget (Persistent, left of scrollable)

                    - Child: MenuEntryWidget (Dynamic)

                        - Shows the desktop entry icon

                        - Shows the desktop entry name

                        - Shows the desktop entry description

                        - Click or Touch: Execute Primary Action

                            - Launch menu entry (if smearor-wrot: with --rotation parameter)

                        - Longpress or Longtouch: Execute Secondary Action

                - Child: MenuEntryWidget (Persistent, left of scrollable)

## MenuEntryWidget

In principle, the MenuEntryWidget renders a single GTK widget.

Challenge:

- Widget: Launch-Desktop-Entry

- Widget: Date / Time

- Widget: Notification

- Widget: MPRIS

These widgets should not be hard-coded, but loaded from a configuration file.

This is a fantastic and extremely well thought-out concept! A touch-optimized, rotatable launcher with smooth scrolling fills a real gap in the Wayland
ecosystem (especially for Linux tablets or convertibles).

Since you want to load the widgets completely dynamically via a configuration file, a simple match on predefined enums is not enough. You need a plugin or
component system where each type of widget encapsulates its own logic.

Here is the architectural plan and the concrete implementation of how to build this system in Rust and GTK 4.


