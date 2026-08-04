/// Macro to generate boilerplate trait implementations for atomic widgets.
///
/// Atomic widgets (weather, audio, mpris) share identical implementations of
/// `MessageHandler`, `AcceptTopic`, `MessageBroadcaster`, `PluginMetaGetter`,
/// `AsRef`, `Plugin`, `WidgetBuilder`, `GraphicRenderer`, and several helper
/// methods. This macro generates all of them, leaving only the struct
/// definition, view enum, `new()` constructor, `update_ui()`,
/// `render_atomic_view()`, and `render_atomic_graphic_data()` to the widget
/// crate.
///
/// # Requirements
///
/// The widget struct must have these fields:
/// - `meta: PluginMeta`
/// - `core_context: Option<FfiCoreContext>`
/// - `config: AtomicWidgetConfig`
/// - `view: <ViewType>` (must be `Copy + Debug`)
/// - `icon_label: Rc<RefCell<Option<Label>>>`
/// - `main_label: Rc<RefCell<Option<Label>>>`
/// - `info_label: Rc<RefCell<Option<Label>>>`
/// - `latest_status: Rc<RefCell<Option<StatusType>>>`
///
/// The widget must also define:
/// - `fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper>`
/// - `fn update_ui(&self, status: &StatusType)`
/// - `fn render_atomic_view(status: &StatusType, view: ViewType) -> (String, String, String)`
/// - `fn render_atomic_graphic_data(&self) -> AtomicGraphicData`
///
/// When `graphic_renderer: true` is set, the widget must also implement
/// `AtomicGraphicRenderer` in a separate impl block.
///
/// # Example
///
/// ```rust
/// atomic_widget_impl! {
///     widget: AudioAtomicWidget,
///     status: AudioStatusMessage,
///     topic: TOPIC_STATUS,
///     debug_tag: "audio-atomic",
///     mcp_description: "Audio atomic widget",
///     css_prefix: "audio",
///     default_icon: '\u{f028}',
///     default_main: "--",
///     default_info: "Loading...",
///     refresh_command: AudioCommandMessage::refresh(),
/// }
/// ```
///
/// With custom rendering:
///
/// ```rust
/// atomic_widget_impl! {
///     widget: MprisAtomicWidget,
///     status: MprisStatusMessage,
///     topic: TOPIC_STATUS,
///     debug_tag: "mpris-atomic",
///     mcp_description: "MPRIS atomic widget",
///     css_prefix: "mpris",
///     default_icon: '\u{f001}',
///     default_main: "--",
///     default_info: "Loading...",
///     refresh_command: MprisCommandMessage::refresh(),
///     graphic_renderer: true,
/// }
/// ```
#[macro_export]
macro_rules! atomic_widget_impl {
    // Arm with graphic_renderer: true — widget implements AtomicGraphicRenderer
    {
        widget: $widget:ident,
        status: $status:ty,
        topic: $topic:expr,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        refresh_command: $request:expr,
        graphic_renderer: true,
    } => {
        $crate::atomic_widget_impl! {
            widget: $widget,
            status: $status,
            topic: $topic,
            debug_tag: $tag,
            mcp_description: $desc,
            css_prefix: $css,
            default_icon: $icon,
            default_main: $main,
            default_info: $info,
            refresh_command: $request,
            graphic_renderer: true,
            _phantom: (),
        }
    };
    // Internal arm: graphic_renderer: true with _phantom terminator (no extra message types)
    {
        widget: $widget:ident,
        status: $status:ty,
        topic: $topic:expr,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        refresh_command: $request:expr,
        graphic_renderer: true,
        _phantom: (),
    } => {
        $crate::atomic_widget_impl!(@body
            $widget, $status, $topic, $tag, $desc, $css, $icon, $main, $info, $request,
            true
        );
    };
    // ── No-status arms: for widgets without a backing service (e.g. Clock) ──
    // Arm without status/topic/refresh_command — no service subscription
    {
        widget: $widget:ident,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
    } => {
        $crate::atomic_widget_impl! {
            widget: $widget,
            debug_tag: $tag,
            mcp_description: $desc,
            css_prefix: $css,
            default_icon: $icon,
            default_main: $main,
            default_info: $info,
            graphic_renderer: false,
            _phantom_no_status: (),
        }
    };
    // Arm without status, with extra_message_types (e.g. PersonalizationStatusMessage)
    {
        widget: $widget:ident,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        extra_message_types: [$($extra:ty),+]
    } => {
        $crate::atomic_widget_impl! {
            widget: $widget,
            debug_tag: $tag,
            mcp_description: $desc,
            css_prefix: $css,
            default_icon: $icon,
            default_main: $main,
            default_info: $info,
            graphic_renderer: false,
            extra_message_types: [$($extra),+],
            _phantom_no_status: (),
        }
    };
    // Arm without status, with graphic_renderer: true
    {
        widget: $widget:ident,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        graphic_renderer: true,
    } => {
        $crate::atomic_widget_impl! {
            widget: $widget,
            debug_tag: $tag,
            mcp_description: $desc,
            css_prefix: $css,
            default_icon: $icon,
            default_main: $main,
            default_info: $info,
            graphic_renderer: true,
            _phantom_no_status: (),
        }
    };
    // Arm without status, with graphic_renderer: true and extra_message_types
    {
        widget: $widget:ident,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        graphic_renderer: true,
        extra_message_types: [$($extra:ty),+]
    } => {
        $crate::atomic_widget_impl! {
            widget: $widget,
            debug_tag: $tag,
            mcp_description: $desc,
            css_prefix: $css,
            default_icon: $icon,
            default_main: $main,
            default_info: $info,
            graphic_renderer: true,
            extra_message_types: [$($extra),+],
            _phantom_no_status: (),
        }
    };
    // Internal arm: no-status, graphic_renderer: false, no extra message types
    {
        widget: $widget:ident,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        graphic_renderer: false,
        _phantom_no_status: (),
    } => {
        $crate::atomic_widget_impl!(@body_no_status
            $widget, $tag, $desc, $css, $icon, $main, $info,
            false
        );
    };
    // Internal arm: no-status, graphic_renderer: false, with extra message types
    {
        widget: $widget:ident,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        graphic_renderer: false,
        extra_message_types: [$($extra:ty),+],
        _phantom_no_status: (),
    } => {
        $crate::atomic_widget_impl!(@body_no_status
            $widget, $tag, $desc, $css, $icon, $main, $info,
            false,
            $($extra),+
        );
    };
    // Internal arm: no-status, graphic_renderer: true, no extra message types
    {
        widget: $widget:ident,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        graphic_renderer: true,
        _phantom_no_status: (),
    } => {
        $crate::atomic_widget_impl!(@body_no_status
            $widget, $tag, $desc, $css, $icon, $main, $info,
            true
        );
    };
    // Internal arm: no-status, graphic_renderer: true, with extra message types
    {
        widget: $widget:ident,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        graphic_renderer: true,
        extra_message_types: [$($extra:ty),+],
        _phantom_no_status: (),
    } => {
        $crate::atomic_widget_impl!(@body_no_status
            $widget, $tag, $desc, $css, $icon, $main, $info,
            true,
            $($extra),+
        );
    };
    // ── End no-status arms ──
    // Arm: no-status, graphic_renderer: true, extra_message_types, span_action_handler: true
    {
        widget: $widget:ident,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        graphic_renderer: true,
        extra_message_types: [$($extra:ty),+],
        span_action_handler: true,
    } => {
        $crate::atomic_widget_impl!(@body_no_status_span
            $widget, $tag, $desc, $css, $icon, $main, $info,
            true,
            $($extra),+
        );
    };
    // Arm with graphic_renderer: false (default) — no custom rendering
    {
        widget: $widget:ident,
        status: $status:ty,
        topic: $topic:expr,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        refresh_command: $request:expr,
    } => {
        $crate::atomic_widget_impl! {
            widget: $widget,
            status: $status,
            topic: $topic,
            debug_tag: $tag,
            mcp_description: $desc,
            css_prefix: $css,
            default_icon: $icon,
            default_main: $main,
            default_info: $info,
            refresh_command: $request,
            graphic_renderer: false,
            _phantom: (),
        }
    };
    // Arm with extra_message_types — widget subscribes to additional message types
    // (e.g. PersonalizationStatusMessage). Topics are derived via MessageTopic::topic().
    {
        widget: $widget:ident,
        status: $status:ty,
        topic: $topic:expr,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        refresh_command: $request:expr,
        extra_message_types: [$($extra:ty),+]
    } => {
        $crate::atomic_widget_impl! {
            widget: $widget,
            status: $status,
            topic: $topic,
            debug_tag: $tag,
            mcp_description: $desc,
            css_prefix: $css,
            default_icon: $icon,
            default_main: $main,
            default_info: $info,
            refresh_command: $request,
            graphic_renderer: false,
            extra_message_types: [$($extra),+],
            _phantom: (),
        }
    };
    // Arm with extra_message_types and graphic_renderer: true
    {
        widget: $widget:ident,
        status: $status:ty,
        topic: $topic:expr,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        refresh_command: $request:expr,
        graphic_renderer: true,
        extra_message_types: [$($extra:ty),+]
    } => {
        $crate::atomic_widget_impl! {
            widget: $widget,
            status: $status,
            topic: $topic,
            debug_tag: $tag,
            mcp_description: $desc,
            css_prefix: $css,
            default_icon: $icon,
            default_main: $main,
            default_info: $info,
            refresh_command: $request,
            graphic_renderer: true,
            extra_message_types: [$($extra),+],
            _phantom: (),
        }
    };
    // Internal arm: extra_message_types with graphic_renderer: false
    {
        widget: $widget:ident,
        status: $status:ty,
        topic: $topic:expr,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        refresh_command: $request:expr,
        graphic_renderer: false,
        extra_message_types: [$($extra:ty),+],
        _phantom: (),
    } => {
        $crate::atomic_widget_impl!(@body
            $widget, $status, $topic, $tag, $desc, $css, $icon, $main, $info, $request,
            false,
            $($extra),+
        );
    };
    // Internal arm: extra_message_types with graphic_renderer: true
    {
        widget: $widget:ident,
        status: $status:ty,
        topic: $topic:expr,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        refresh_command: $request:expr,
        graphic_renderer: true,
        extra_message_types: [$($extra:ty),+],
        _phantom: (),
    } => {
        $crate::atomic_widget_impl!(@body
            $widget, $status, $topic, $tag, $desc, $css, $icon, $main, $info, $request,
            true,
            $($extra),+
        );
    };
    // Internal arm: graphic_renderer: false with _phantom terminator (no extra message types)
    {
        widget: $widget:ident,
        status: $status:ty,
        topic: $topic:expr,
        debug_tag: $tag:literal,
        mcp_description: $desc:literal,
        css_prefix: $css:literal,
        default_icon: $icon:expr,
        default_main: $main:literal,
        default_info: $info:literal,
        refresh_command: $request:expr,
        graphic_renderer: false,
        _phantom: (),
    } => {
        $crate::atomic_widget_impl!(@body
            $widget, $status, $topic, $tag, $desc, $css, $icon, $main, $info, $request,
            false
        );
    };
    // Internal body — generates all trait impls including GraphicRenderer (with custom renderer)
    (@body
        $widget:ident, $status:ty, $topic:expr, $tag:literal, $desc:literal,
        $css:literal, $icon:expr, $main:literal, $info:literal, $request:expr,
        true
        $(, $extra:ty)*
    ) => {
        $crate::atomic_widget_impl!(@common_body
            $widget, $status, $topic, $tag, $desc, $css, $icon, $main, $info, $request,
            $($extra),*
        );

        impl smearor_swipe_launcher_plugin_api::GraphicRenderer for $widget {
            fn render_graphic(&self, width: u32, height: u32) -> smearor_swipe_launcher_plugin_api::FfiGraphic {
                tracing::trace!("{} ({:?}): render_graphic {}x{}", $tag, self.view, width, height);

                let mut pixels = vec![0u8; (width * height * 4) as usize];
                let data = self.render_atomic_graphic_data();

                smearor_swipe_launcher_plugin_api::render_atomic_graphic_default(
                    &mut pixels,
                    width,
                    height,
                    data.icon_char,
                    &data.main_text,
                    &data.info_text,
                    data.is_error,
                    &self.config,
                    Some(self as &dyn smearor_swipe_launcher_plugin_api::AtomicGraphicRenderer),
                    data.icon_color,
                    data.main_text_color,
                    data.info_text_color,
                );

                smearor_swipe_launcher_plugin_api::FfiGraphic::from_pixels(width, height, pixels)
            }
        }
    };
    // Internal body — generates all trait impls including GraphicRenderer (no custom renderer)
    (@body
        $widget:ident, $status:ty, $topic:expr, $tag:literal, $desc:literal,
        $css:literal, $icon:expr, $main:literal, $info:literal, $request:expr,
        false
        $(, $extra:ty)*
    ) => {
        $crate::atomic_widget_impl!(@common_body
            $widget, $status, $topic, $tag, $desc, $css, $icon, $main, $info, $request,
            $($extra),*
        );

        impl smearor_swipe_launcher_plugin_api::GraphicRenderer for $widget {
            fn render_graphic(&self, width: u32, height: u32) -> smearor_swipe_launcher_plugin_api::FfiGraphic {
                tracing::trace!("{} ({:?}): render_graphic {}x{}", $tag, self.view, width, height);

                let mut pixels = vec![0u8; (width * height * 4) as usize];
                let data = self.render_atomic_graphic_data();

                smearor_swipe_launcher_plugin_api::render_atomic_graphic_default(
                    &mut pixels,
                    width,
                    height,
                    data.icon_char,
                    &data.main_text,
                    &data.info_text,
                    data.is_error,
                    &self.config,
                    None::<&dyn smearor_swipe_launcher_plugin_api::AtomicGraphicRenderer>,
                    data.icon_color,
                    data.main_text_color,
                    data.info_text_color,
                );

                smearor_swipe_launcher_plugin_api::FfiGraphic::from_pixels(width, height, pixels)
            }
        }
    };
    // Internal body for no-status widgets — generates GraphicRenderer + common_body_no_status
    (@body_no_status
        $widget:ident, $tag:literal, $desc:literal,
        $css:literal, $icon:expr, $main:literal, $info:literal,
        true
        $(, $extra:ty)*
    ) => {
        $crate::atomic_widget_impl!(@common_body_no_status
            $widget, $tag, $desc, $css, $icon, $main, $info,
            $($extra),*
        );

        impl smearor_swipe_launcher_plugin_api::GraphicRenderer for $widget {
            fn render_graphic(&self, width: u32, height: u32) -> smearor_swipe_launcher_plugin_api::FfiGraphic {
                tracing::trace!("{} ({:?}): render_graphic {}x{}", $tag, self.view, width, height);

                let mut pixels = vec![0u8; (width * height * 4) as usize];
                let data = self.render_atomic_graphic_data();

                smearor_swipe_launcher_plugin_api::render_atomic_graphic_default(
                    &mut pixels,
                    width,
                    height,
                    data.icon_char,
                    &data.main_text,
                    &data.info_text,
                    data.is_error,
                    &self.config,
                    Some(self as &dyn smearor_swipe_launcher_plugin_api::AtomicGraphicRenderer),
                    data.icon_color,
                    data.main_text_color,
                    data.info_text_color,
                );

                smearor_swipe_launcher_plugin_api::FfiGraphic::from_pixels(width, height, pixels)
            }
        }
    };
    // Internal body for no-status widgets — GraphicRenderer without custom renderer
    (@body_no_status
        $widget:ident, $tag:literal, $desc:literal,
        $css:literal, $icon:expr, $main:literal, $info:literal,
        false
        $(, $extra:ty)*
    ) => {
        $crate::atomic_widget_impl!(@common_body_no_status
            $widget, $tag, $desc, $css, $icon, $main, $info,
            $($extra),*
        );

        impl smearor_swipe_launcher_plugin_api::GraphicRenderer for $widget {
            fn render_graphic(&self, width: u32, height: u32) -> smearor_swipe_launcher_plugin_api::FfiGraphic {
                tracing::trace!("{} ({:?}): render_graphic {}x{}", $tag, self.view, width, height);

                let mut pixels = vec![0u8; (width * height * 4) as usize];
                let data = self.render_atomic_graphic_data();

                smearor_swipe_launcher_plugin_api::render_atomic_graphic_default(
                    &mut pixels,
                    width,
                    height,
                    data.icon_char,
                    &data.main_text,
                    &data.info_text,
                    data.is_error,
                    &self.config,
                    None::<&dyn smearor_swipe_launcher_plugin_api::AtomicGraphicRenderer>,
                    data.icon_color,
                    data.main_text_color,
                    data.info_text_color,
                );

                smearor_swipe_launcher_plugin_api::FfiGraphic::from_pixels(width, height, pixels)
            }
        }
    };
    // Common body for no-status widgets — shared trait impls (no MessageHandler<$status>, no request_initial_status)
    (@common_body_no_status
        $widget:ident, $tag:literal, $desc:literal,
        $css:literal, $icon:expr, $main:literal, $info:literal,
        $($extra:ty),*
    ) => {
        impl smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator for $widget {
            fn register_mcp_capabilities(&self) {
                if self.config.description.is_some() {
                    let tool = smearor_model_mcp::RegisterToolMessage::new(
                        &format!("button_{}", self.meta.id),
                        self.config.description.as_deref().unwrap_or($desc),
                        r#"{ "type": "object", "properties": { "action": { "type": "string", "enum": ["click", "longpress", "hold_start", "hold_stop", "double_press", "compound_longpress"], "description": "The action to trigger" } }, "required": ["action"] }"#,
                    );
                    smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(tool);
                }
            }
        }

        impl $widget {
            pub fn broadcast_widget_update(&self) {
                let plugin_id = self.meta.id.to_string();
                let msg = smearor_model_widget::WidgetUpdateMessage::new(&plugin_id, "");
                smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(msg);
            }

            pub fn dispatch_action(&self, action: smearor_swipe_launcher_plugin_api::AtomicAction) {
                self.config.dispatch_action(&smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self), action);
            }
        }

        impl smearor_swipe_launcher_plugin_api::MessageHandler<smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage>> for $widget {
            fn handle_message(&self, message: smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage>, _sender_id: &str) {
                let tool_name = message.0.name.to_string();
                let arguments = message.0.arguments.to_string();
                tracing::trace!("{}: InvokeToolMessage name={} args={}", $tag, tool_name, arguments);

                let own_button_name = format!("button_{}", self.meta.id);
                if tool_name == own_button_name {
                    let action_str = serde_json::from_str::<serde_json::Value>(&arguments)
                        .ok()
                        .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                        .unwrap_or_else(|| "click".to_string());
                    let action = <smearor_swipe_launcher_plugin_api::AtomicAction as std::str::FromStr>::from_str(&action_str)
                        .unwrap_or(smearor_swipe_launcher_plugin_api::AtomicAction::Click);

                    self.dispatch_action(action);
                    let response = smearor_model_mcp::InvokeToolResponse::success(&message.0.correlation_id, &format!("{} handled", action.as_ref()));
                    smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(response);
                }
            }
        }

        impl smearor_swipe_launcher_plugin_api::AcceptTopic<smearor_swipe_launcher_plugin_api::FfiEnvelope> for $widget {
            fn accept_topic(&self, topic: &str) -> bool {
                topic == smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL
                    $( || topic == <$extra as smearor_swipe_launcher_plugin_api::MessageTopic>::topic() )*
            }
        }

        impl smearor_swipe_launcher_plugin_api::MessageBroadcaster for $widget {}

        impl smearor_swipe_launcher_plugin_api::PluginMetaGetter for $widget {
            fn meta(&self) -> smearor_swipe_launcher_plugin_api::PluginMeta {
                self.meta.clone()
            }
        }

        impl AsRef<Option<smearor_swipe_launcher_plugin_api::FfiCoreContext>> for $widget {
            fn as_ref(&self) -> &Option<smearor_swipe_launcher_plugin_api::FfiCoreContext> {
                &self.core_context
            }
        }

        impl smearor_swipe_launcher_plugin_api::WidgetPlugin for $widget {
            fn on_message(&mut self, message: *mut core::ffi::c_void) {
                if !message.is_null() {
                    unsafe {
                        let envelope = &*(message as *mut smearor_swipe_launcher_plugin_api::FfiEnvelope);
                        let topic = envelope.topic.to_string();
                        tracing::trace!("{}: on_message topic={} type_id={}", $tag, topic, envelope.type_id);
                        if envelope.type_id == <smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage> as smearor_swipe_launcher_plugin_api::TypedMessage>::TYPE_ID {
                            smearor_swipe_launcher_plugin_api::MessageHandler::<smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage>>::handle_envelope_message(self, envelope);
                        } $( else if envelope.type_id == <$extra as smearor_swipe_launcher_plugin_api::TypedMessage>::TYPE_ID {
                            smearor_swipe_launcher_plugin_api::MessageHandler::<$extra>::handle_envelope_message(self, envelope);
                        } )*
                    }
                }
            }
        }

        impl smearor_swipe_launcher_plugin_api::WidgetBuilder for $widget {
            fn build_widget(&mut self) -> gtk4::Widget {
                let params = smearor_swipe_launcher_plugin_api::AtomicWidgetBuildParams {
                    css_prefix: $css,
                    default_icon: $icon,
                    default_main: $main,
                    default_info: $info,
                };
                let (widget, icon_label, main_label, info_label) = smearor_swipe_launcher_plugin_api::build_atomic_widget(&smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self), &self.config, &params);

                *self.icon_label.borrow_mut() = Some(icon_label);
                *self.main_label.borrow_mut() = Some(main_label);
                *self.info_label.borrow_mut() = Some(info_label);

                self.update_ui();

                smearor_swipe_launcher_plugin_api::apply_widget_css_classes(&widget, &self.meta.id, &[]);
                widget
            }
        }
    };
    // Common body — shared trait impls (everything except GraphicRenderer)
    (@common_body
        $widget:ident, $status:ty, $topic:expr, $tag:literal, $desc:literal,
        $css:literal, $icon:expr, $main:literal, $info:literal, $request:expr,
        $($extra:ty),*
    ) => {
        impl smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator for $widget {
            fn register_mcp_capabilities(&self) {
                if self.config.description.is_some() {
                    let tool = smearor_model_mcp::RegisterToolMessage::new(
                        &format!("button_{}", self.meta.id),
                        self.config.description.as_deref().unwrap_or($desc),
                        r#"{ "type": "object", "properties": { "action": { "type": "string", "enum": ["click", "longpress", "hold_start", "hold_stop", "double_press", "compound_longpress"], "description": "The action to trigger" } }, "required": ["action"] }"#,
                    );
                    smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(tool);
                }
            }
        }

        impl $widget {
            pub fn broadcast_widget_update(&self) {
                let plugin_id = self.meta.id.to_string();
                let msg = smearor_model_widget::WidgetUpdateMessage::new(&plugin_id, "");
                smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(msg);
            }

            pub fn request_initial_status(&self) {
                smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic($request);
            }

            pub fn dispatch_action(&self, action: smearor_swipe_launcher_plugin_api::AtomicAction) {
                self.config.dispatch_action(&smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self), action);
            }
        }

        impl smearor_swipe_launcher_plugin_api::MessageHandler<$status> for $widget {
            fn handle_message(&self, message: $status, _sender_id: &str) {
                *self.latest_status.borrow_mut() = Some(message.clone());
                self.update_ui(&message);
                self.broadcast_widget_update();
            }
        }

        impl smearor_swipe_launcher_plugin_api::MessageHandler<smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage>> for $widget {
            fn handle_message(&self, message: smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage>, _sender_id: &str) {
                let tool_name = message.0.name.to_string();
                let arguments = message.0.arguments.to_string();
                tracing::trace!("{}: InvokeToolMessage name={} args={}", $tag, tool_name, arguments);

                let own_button_name = format!("button_{}", self.meta.id);
                if tool_name == own_button_name {
                    let action_str = serde_json::from_str::<serde_json::Value>(&arguments)
                        .ok()
                        .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                        .unwrap_or_else(|| "click".to_string());
                    let action = <smearor_swipe_launcher_plugin_api::AtomicAction as std::str::FromStr>::from_str(&action_str)
                        .unwrap_or(smearor_swipe_launcher_plugin_api::AtomicAction::Click);

                    self.dispatch_action(action);
                    let response = smearor_model_mcp::InvokeToolResponse::success(&message.0.correlation_id, &format!("{} handled", action.as_ref()));
                    smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(response);
                }
            }
        }

        impl smearor_swipe_launcher_plugin_api::AcceptTopic<smearor_swipe_launcher_plugin_api::FfiEnvelope> for $widget {
            fn accept_topic(&self, topic: &str) -> bool {
                topic == $topic
                    || topic == smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL
                    $( || topic == <$extra as smearor_swipe_launcher_plugin_api::MessageTopic>::topic() )*
            }
        }

        impl smearor_swipe_launcher_plugin_api::MessageBroadcaster for $widget {}

        impl smearor_swipe_launcher_plugin_api::PluginMetaGetter for $widget {
            fn meta(&self) -> smearor_swipe_launcher_plugin_api::PluginMeta {
                self.meta.clone()
            }
        }

        impl AsRef<Option<smearor_swipe_launcher_plugin_api::FfiCoreContext>> for $widget {
            fn as_ref(&self) -> &Option<smearor_swipe_launcher_plugin_api::FfiCoreContext> {
                &self.core_context
            }
        }

        impl smearor_swipe_launcher_plugin_api::WidgetPlugin for $widget {
            fn on_message(&mut self, message: *mut core::ffi::c_void) {
                if !message.is_null() {
                    unsafe {
                        let envelope = &*(message as *mut smearor_swipe_launcher_plugin_api::FfiEnvelope);
                        let topic = envelope.topic.to_string();
                        tracing::trace!("{}: on_message topic={} type_id={}", $tag, topic, envelope.type_id);
                        if envelope.type_id == <$status as smearor_swipe_launcher_plugin_api::TypedMessage>::TYPE_ID {
                            smearor_swipe_launcher_plugin_api::MessageHandler::<$status>::handle_envelope_message(self, envelope);
                        } else if envelope.type_id == <smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage> as smearor_swipe_launcher_plugin_api::TypedMessage>::TYPE_ID {
                            smearor_swipe_launcher_plugin_api::MessageHandler::<smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage>>::handle_envelope_message(self, envelope);
                        } $( else if envelope.type_id == <$extra as smearor_swipe_launcher_plugin_api::TypedMessage>::TYPE_ID {
                            smearor_swipe_launcher_plugin_api::MessageHandler::<$extra>::handle_envelope_message(self, envelope);
                        } )*
                    }
                }
            }
        }

        impl smearor_swipe_launcher_plugin_api::WidgetBuilder for $widget {
            fn build_widget(&mut self) -> gtk4::Widget {
                let params = smearor_swipe_launcher_plugin_api::AtomicWidgetBuildParams {
                    css_prefix: $css,
                    default_icon: $icon,
                    default_main: $main,
                    default_info: $info,
                };
                let (widget, icon_label, main_label, info_label) = smearor_swipe_launcher_plugin_api::build_atomic_widget(&smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self), &self.config, &params);

                *self.icon_label.borrow_mut() = Some(icon_label);
                *self.main_label.borrow_mut() = Some(main_label);
                *self.info_label.borrow_mut() = Some(info_label);

                if let Some(status) = self.latest_status.borrow().as_ref() {
                    self.update_ui(status);
                }
                self.request_initial_status();

                smearor_swipe_launcher_plugin_api::apply_widget_css_classes(&widget, &self.meta.id, &[]);
                widget
            }
        }
    };
    // Internal body for no-status widgets with span_action_handler — generates GraphicRenderer + span-aware common_body
    (@body_no_status_span
        $widget:ident, $tag:literal, $desc:literal,
        $css:literal, $icon:expr, $main:literal, $info:literal,
        true
        $(, $extra:ty)*
    ) => {
        $crate::atomic_widget_impl!(@common_body_no_status_span
            $widget, $tag, $desc, $css, $icon, $main, $info,
            $($extra),*
        );

        impl smearor_swipe_launcher_plugin_api::GraphicRenderer for $widget {
            fn render_graphic(&self, width: u32, height: u32) -> smearor_swipe_launcher_plugin_api::FfiGraphic {
                tracing::trace!("{} ({:?}): render_graphic {}x{}", $tag, self.view, width, height);

                let mut pixels = vec![0u8; (width * height * 4) as usize];
                let data = self.render_atomic_graphic_data();

                smearor_swipe_launcher_plugin_api::render_atomic_graphic_default(
                    &mut pixels,
                    width,
                    height,
                    data.icon_char,
                    &data.main_text,
                    &data.info_text,
                    data.is_error,
                    &self.config,
                    Some(self as &dyn smearor_swipe_launcher_plugin_api::AtomicGraphicRenderer),
                    data.icon_color,
                    data.main_text_color,
                    data.info_text_color,
                );

                smearor_swipe_launcher_plugin_api::FfiGraphic::from_pixels(width, height, pixels)
            }
        }
    };
    // Common body for no-status widgets with span_action_handler — same as @common_body_no_status
    // but the MessageHandler<InvokeToolMessage> calls SpanActionHandler::on_span_action before dispatch_action
    (@common_body_no_status_span
        $widget:ident, $tag:literal, $desc:literal,
        $css:literal, $icon:expr, $main:literal, $info:literal,
        $($extra:ty),*
    ) => {
        impl smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator for $widget {
            fn register_mcp_capabilities(&self) {
                if self.config.description.is_some() {
                    let tool = smearor_model_mcp::RegisterToolMessage::new(
                        &format!("button_{}", self.meta.id),
                        self.config.description.as_deref().unwrap_or($desc),
                        r#"{ "type": "object", "properties": { "action": { "type": "string", "enum": ["click", "longpress", "hold_start", "hold_stop", "double_press", "compound_longpress"], "description": "The action to trigger" } }, "required": ["action"] }"#,
                    );
                    smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(tool);
                }
            }
        }

        impl $widget {
            pub fn broadcast_widget_update(&self) {
                let plugin_id = self.meta.id.to_string();
                let msg = smearor_model_widget::WidgetUpdateMessage::new(&plugin_id, "");
                smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(msg);
            }

            pub fn dispatch_action(&self, action: smearor_swipe_launcher_plugin_api::AtomicAction) {
                self.config.dispatch_action(&smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self), action);
            }
        }

        impl smearor_swipe_launcher_plugin_api::MessageHandler<smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage>> for $widget {
            fn handle_message(&self, message: smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage>, _sender_id: &str) {
                let tool_name = message.0.name.to_string();
                let arguments = message.0.arguments.to_string();
                tracing::trace!("{}: InvokeToolMessage name={} args={}", $tag, tool_name, arguments);

                let own_button_name = format!("button_{}", self.meta.id);
                if tool_name == own_button_name {
                    let action_str = serde_json::from_str::<serde_json::Value>(&arguments)
                        .ok()
                        .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                        .unwrap_or_else(|| "click".to_string());
                    let action = <smearor_swipe_launcher_plugin_api::AtomicAction as std::str::FromStr>::from_str(&action_str)
                        .unwrap_or(smearor_swipe_launcher_plugin_api::AtomicAction::Click);

                    smearor_swipe_launcher_plugin_api::SpanActionHandler::on_span_action(self, action, self.span_index);
                    self.dispatch_action(action);
                    let response = smearor_model_mcp::InvokeToolResponse::success(&message.0.correlation_id, &format!("{} handled", action.as_ref()));
                    smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(response);
                }
            }
        }

        impl smearor_swipe_launcher_plugin_api::AcceptTopic<smearor_swipe_launcher_plugin_api::FfiEnvelope> for $widget {
            fn accept_topic(&self, topic: &str) -> bool {
                topic == smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL
                    $( || topic == <$extra as smearor_swipe_launcher_plugin_api::MessageTopic>::topic() )*
            }
        }

        impl smearor_swipe_launcher_plugin_api::MessageBroadcaster for $widget {}

        impl smearor_swipe_launcher_plugin_api::PluginMetaGetter for $widget {
            fn meta(&self) -> smearor_swipe_launcher_plugin_api::PluginMeta {
                self.meta.clone()
            }
        }

        impl AsRef<Option<smearor_swipe_launcher_plugin_api::FfiCoreContext>> for $widget {
            fn as_ref(&self) -> &Option<smearor_swipe_launcher_plugin_api::FfiCoreContext> {
                &self.core_context
            }
        }

        impl smearor_swipe_launcher_plugin_api::WidgetPlugin for $widget {
            fn on_message(&mut self, message: *mut core::ffi::c_void) {
                if !message.is_null() {
                    unsafe {
                        let envelope = &*(message as *mut smearor_swipe_launcher_plugin_api::FfiEnvelope);
                        let topic = envelope.topic.to_string();
                        tracing::trace!("{}: on_message topic={} type_id={}", $tag, topic, envelope.type_id);
                        if envelope.type_id == <smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage> as smearor_swipe_launcher_plugin_api::TypedMessage>::TYPE_ID {
                            smearor_swipe_launcher_plugin_api::MessageHandler::<smearor_swipe_launcher_plugin_api::FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage>>::handle_envelope_message(self, envelope);
                        } $( else if envelope.type_id == <$extra as smearor_swipe_launcher_plugin_api::TypedMessage>::TYPE_ID {
                            smearor_swipe_launcher_plugin_api::MessageHandler::<$extra>::handle_envelope_message(self, envelope);
                        } )*
                    }
                }
            }
        }

        impl smearor_swipe_launcher_plugin_api::WidgetBuilder for $widget {
            fn build_widget(&mut self) -> gtk4::Widget {
                let params = smearor_swipe_launcher_plugin_api::AtomicWidgetBuildParams {
                    css_prefix: $css,
                    default_icon: $icon,
                    default_main: $main,
                    default_info: $info,
                };
                let (widget, icon_label, main_label, info_label) = smearor_swipe_launcher_plugin_api::build_atomic_widget(&smearor_swipe_launcher_plugin_api::MessageBroadcaster::get_broadcaster(self), &self.config, &params);

                *self.icon_label.borrow_mut() = Some(icon_label);
                *self.main_label.borrow_mut() = Some(main_label);
                *self.info_label.borrow_mut() = Some(info_label);

                self.update_ui();

                smearor_swipe_launcher_plugin_api::apply_widget_css_classes(&widget, &self.meta.id, &[]);
                widget
            }
        }
    };
}
