# Rotation

The launcher supports rotation at 0°, 90°, 180°, and 270°. This is essential for table-top installations where each side of the table needs a launcher facing
the user.

## How Rotation Works

Rotation affects three things:

1. **Visual orientation** — The `RotationWidget` from `smearor-wrot-rotation` rotates the entire widget tree
2. **Layer-shell position** — The window anchors to a different screen edge depending on rotation
3. **Input coordinate mapping** — Touch and mouse coordinates are transformed to match the rotated layout

## Rotation Degrees and Positions

| Rotation | Position | Edge                      |
|----------|----------|---------------------------|
| 0°       | Bottom   | Layer-shell bottom anchor |
| 90°      | Left     | Layer-shell left anchor   |
| 180°     | Top      | Layer-shell top anchor    |
| 270°     | Right    | Layer-shell right anchor  |

## Widget Tree

```mermaid
graph TB
    Window["ApplicationWindow (layer-shell)"]
    Rotation["RotationWidget"]
    Swipe["SwipeWidget"]
    Areas["Area Container"]

    Window --> Rotation
    Rotation --> Swipe
    Swipe --> Areas
```

## Configuration

Rotation is set in `config.toml`:

```toml
[launcher]
rotation = 0
```

## App Launch with Rotation

When launching applications via `smearor-wrot`, the rotation parameter is passed so that the launched app opens with the correct orientation:

```toml
[my_app]
defaults = "app_launcher"
desktop_file_path = "/usr/share/applications/myapp.desktop"
[my_app.wrapper]
follows_rotation = true
```

## Runtime Rotation

The launcher can respond to rotation changes at runtime. When the compositor signals a monitor change or rotation event, the launcher adjusts its layer-shell
anchors and re-renders the widget tree.
