# Glossary

| Term                  | Definition                                                                                                         |
|-----------------------|--------------------------------------------------------------------------------------------------------------------|
| **Area**              | A layout region in the launcher (fixed or scroll) that contains plugins                                            |
| **AreaManager**       | Per-instance component that manages areas, transitions, and transient areas                                        |
| **Action Binding**    | A configurable mapping from a user interaction (click, long-press, etc.) to a broker message                       |
| **Atomic Widget**     | A MacroPad widget that renders multiple buttons as a single combined image, then splits into individual key images |
| **Broker**            | Short for Message Broker — the central message routing hub                                                         |
| **FfiCoreContext**    | Context passed to plugins at construction, providing broker handle, executor, and JSON converter registration      |
| **FfiEnvelope**       | ABI-safe message envelope carrying topic, sender, target, type ID, and raw payload pointer                         |
| **FfiGraphic**        | RGBA pixel buffer returned by `GraphicRenderer::render_graphic()` for headless rendering                           |
| **GraphicRenderer**   | Trait for rendering widgets to RGBA pixel buffers (headless instances)                                             |
| **Headless Instance** | A launcher instance without a GTK window, used for MacroPad devices                                                |
| **Instance**          | Short for LauncherInstance — a per-window launcher with its own plugins and areas                                  |
| **Layer Shell**       | Wayland protocol for creating desktop panels/overlays without window decorations                                   |
| **LauncherHost**      | The single host process that manages all instances, services, and the message broker                               |
| **MacroPad**          | Compact peripheral input device with LCD keys (e.g. Stream Deck, Loupedeck)                                        |
| **MCP**               | Model Context Protocol — standard for exposing tools, resources, and prompts to AI clients                         |
| **Message Broker**    | Central communication hub that routes `FfiEnvelope` messages by topic and target instance                          |
| **Model Crate**       | A crate in `model/` that defines shared structs, enums, and message types                                          |
| **Plugin**            | A dynamically loaded library (`.so`) that provides widgets or services                                             |
| **PluginManager**     | Per-instance component that loads, unloads, and manages widget plugins                                             |
| **ReAct**             | Reason + Act — LLM reasoning loop where the model selects and invokes tools                                        |
| **Rotation**          | Visual rotation (0°, 90°, 180°, 270°) for table-top installations                                                  |
| **Service**           | A dynamically loaded library that implements business logic without UI                                             |
| **ServiceManager**    | Singleton component that loads, unloads, and manages service plugins                                               |
| **Span Group**        | A MacroPad layout concept where a single logical button spans multiple physical keys                               |
| **stabby**            | ABI-stable trait object library used for FFI-safe plugin VTables                                                   |
| **Topic**             | A string that identifies the type of a broker message (e.g. `service.audio.command`)                               |
| **Transient Area**    | An area that auto-closes when the user clicks outside or presses escape                                            |
| **WebRenderer**       | Trait for rendering widgets to HTML fragments (web instances)                                                      |
| **Widget Plugin**     | A plugin that provides GTK widgets and handles user input                                                          |
| **WidgetBuilder**     | Trait for building GTK widgets (`build_widget(rotation) -> gtk4::Widget`)                                          |
