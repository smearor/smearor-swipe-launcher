# MacroPad Models

The launcher supports MacroPad hardware from three manufacturers: Elgato, Loupedeck, and Razer. All are treated as headless launcher instances.

## What Are MacroPads?

Macro pads are compact peripheral input devices designed for fast, tactile access to frequently used actions. Each key or pad is typically paired with a small
LCD display that can show a custom icon, label, or animation. Pressing a key triggers a configurable action.

### Core Concepts

- **LCD keys** — Physical buttons with an embedded LCD screen. Software renders custom icons or text to each key individually.
- **Dial/knob encoders** — Rotatable controls that can be turned (for incremental adjustments like volume) and pressed (for a click action).
- **Touch elements** — Capacitive touch strips or touchscreens that support tap, swipe, or multi-touch gestures.
- **Sub-menus** — Pressing a button can switch the entire key layout to a new set of actions, similar to a folder structure.
- **Brightness control** — Adjustable backlight brightness for the key displays.

## Elgato Stream Deck

| Model           | LCD Keys   | Dials          | Touch Elements | Additional Displays     |
|-----------------|------------|----------------|----------------|-------------------------|
| Mini            | 6          | —              | —              | —                       |
| Neo             | 8          | —              | 2 touch points | 1 infobar               |
| MK.2 / Standard | 15         | —              | —              | —                       |
| XL              | 32         | —              | —              | —                       |
| Stream Deck +   | 8          | 4 (with click) | 1 touch strip  | 1 touch display         |
| Pedal           | 3 (pedals) | —              | —              | —                       |
| Studio          | 32         | 2 (with click) | NFC sensor     | 1 infobar + 2 LED rings |

## Loupedeck

| Model      | Touch Buttons | Dials            | Physical Keys | Touch Displays                        |
|------------|---------------|------------------|---------------|---------------------------------------|
| Live S     | 15            | 2 (with click)   | 4 (RGB)       | 1× touchscreen                        |
| Live       | 12            | 6 (with click)   | 8 (RGB)       | 1× touchscreen                        |
| CT         | 12            | 6 (with click)   | 20 (RGB)      | 1× touchscreen + 1× round touchscreen |
| Loupedeck+ | —             | 14 (dials/wheel) | 25+           | No displays (status LEDs only)        |

## Razer

| Model               | LCD Touch Buttons | Physical LCD Buttons | Dials | Extra Keys | Displays                      |
|---------------------|-------------------|----------------------|-------|------------|-------------------------------|
| Stream Controller   | 12                | —                    | 6     | 8 (RGB)    | 1× touchscreen + 2× side LCDs |
| Stream Controller X | —                 | 15                   | —     | —          | 15× individual LCDs           |

## Comparison

| Feature         | Stream Deck            | Loupedeck                    | Razer                             |
|-----------------|------------------------|------------------------------|-----------------------------------|
| LCD keys        | Yes (all except Pedal) | Touch buttons                | SC X: LCD keys; SC: touch buttons |
| Clickable dials | SD+, Studio            | All except Loupedeck+        | Stream Controller only            |
| Physical keys   | Pedal only             | Live S, Live, CT, Loupedeck+ | SC (8 RGB), SC X (15 LCD)         |
| Touchscreen     | SD+, Neo (infobar)     | Live S, Live, CT             | Stream Controller                 |
| RGB lighting    | —                      | Physical keys (RGB)          | Chroma RGB (keys)                 |
| NFC             | Studio only            | —                            | —                                 |
| Pedal input     | Pedal only             | —                            | —                                 |

## Integration

The launcher treats MacroPads as **headless launcher instances**. See [MacroPad Integration](../features/macropad.md) for the feature overview
and [Instance Types](../architecture/instance-types.md) for technical details.

### Supported Drivers

- **Stream Deck** (`services/streamdeck`) — USB HID via `elgato-streamdeck` crate
- **Loupedeck** (`services/loupedeck`) — Serial via `loupedeck-driver` crate

### Udev Rules

- `resources/udev/52-streamdeck.rules`
- `resources/udev/52-loupedeck.rules`
