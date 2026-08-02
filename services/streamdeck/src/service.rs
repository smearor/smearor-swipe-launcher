use crate::config::StreamDeckConfig;
use elgato_streamdeck::StreamDeck;
use elgato_streamdeck::StreamDeckInput;
use elgato_streamdeck::list_devices;
use elgato_streamdeck::new_hidapi;
use smearor_model_macropad::DeviceCommand;
use smearor_model_macropad::DimmingConfig;
use smearor_model_macropad::DimmingPhase;
use smearor_model_macropad::DimmingState;
use smearor_model_macropad::MacroPadCommand;
use smearor_model_macropad::MacroPadCommandMessage;
use smearor_model_macropad::MacroPadCommandType;
use smearor_model_macropad::MacroPadConnectionStatus;
use smearor_model_macropad::MacroPadInputMessage;
use smearor_model_macropad::TOPIC_MACROPAD_COMMAND;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::ServicePlugin;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use tracing::debug;
use tracing::error;
use tracing::trace;

/// Resolved per-device configuration after merging global config with device overrides.
struct ResolvedDeviceConfig {
    brightness: u8,
    dimming: DimmingConfig,
}

/// Resolve per-device configuration by merging global config with device overrides.
fn resolve_device_config(serial: &str, global: &StreamDeckConfig) -> ResolvedDeviceConfig {
    let trimmed = serial.trim();
    let override_entry = global.device_overrides.iter().find(|o| o.serial.trim() == trimmed);

    match override_entry {
        Some(o) => ResolvedDeviceConfig {
            brightness: o.brightness.unwrap_or(global.brightness),
            dimming: o.dimming.merge(&global.dimming),
        },
        None => ResolvedDeviceConfig {
            brightness: global.brightness,
            dimming: global.dimming.clone(),
        },
    }
}

pub struct StreamDeckService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    #[allow(unused)]
    pub config: StreamDeckConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<(String, DeviceCommand)>,
}

impl StreamDeckService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let service_config = StreamDeckConfig::parse(&config.config)
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;

        let meta = PluginMeta::try_from(&config)?;
        let meta_clone = meta.clone();
        let core_context_clone = core_context;
        let service_config_clone = service_config.clone();

        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<(String, DeviceCommand)>();

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(e) => {
                    error!("StreamDeck service: failed to create tokio runtime: {e}");
                    return;
                }
            };
            runtime.block_on(async move {
                run_device_loop(service_config_clone, command_receiver, meta_clone, core_context_clone).await;
            });
        });

        let service = StreamDeckService {
            meta,
            core_context,
            config: service_config,
            command_sender,
        };
        service.register_mcp_capabilities();
        Ok(service)
    }
}

async fn run_device_loop(
    config: StreamDeckConfig,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<(String, DeviceCommand)>,
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
) {
    debug!("StreamDeck service: starting device loop for instance '{}'", meta.id);

    let hid = match new_hidapi() {
        Ok(hid) => hid,
        Err(e) => {
            error!("StreamDeck service: failed to create HidApi instance: {e}");
            return;
        }
    };

    let devices = list_devices(&hid);
    if devices.is_empty() {
        trace!("StreamDeck service: no Stream Deck devices found");
        return;
    }

    // Connect to all discovered devices and spawn a thread per device.
    struct DeviceEntry {
        serial: String,
        device_type: String,
        key_count: u8,
        key_columns: u8,
        key_width: u32,
        key_height: u32,
        sender: tokio::sync::mpsc::UnboundedSender<DeviceCommand>,
    }
    let mut device_entries: Vec<DeviceEntry> = Vec::new();
    let mut device_threads: Vec<std::thread::JoinHandle<()>> = Vec::new();

    for (kind, serial) in &devices {
        trace!("StreamDeck service: found device {:?} with serial {}", kind, serial);

        let device = match StreamDeck::connect(&hid, *kind, serial) {
            Ok(device) => device,
            Err(e) => {
                error!("StreamDeck service: failed to connect to device {}: {e}", serial);
                continue;
            }
        };

        let device_kind = device.kind();
        let device_type = format!("{:?}", device_kind);
        let key_count = device_kind.key_count() as u8;
        let key_columns = device_kind.column_count() as u8;
        let (img_w, img_h) = device_kind.key_image_format().size;
        let key_width = img_w as u32;
        let key_height = img_h as u32;

        let resolved = resolve_device_config(serial, &config);

        if let Err(e) = device.set_brightness(resolved.brightness) {
            error!("StreamDeck service: failed to set brightness for {}: {e}", serial);
        }

        // Broadcast connection status for this device.
        broadcast_connection(&meta, &core_context, serial, &device_type, key_count, key_columns, key_width, key_height, true);

        let dimming = DimmingState {
            enabled: resolved.dimming.auto_dimming_enabled,
            target_brightness: resolved.brightness,
            dim_brightness: resolved.dimming.auto_dim_brightness,
            idle_timeout: std::time::Duration::from_millis(resolved.dimming.auto_dim_timeout_ms),
            fade_step_duration: std::time::Duration::from_millis(resolved.dimming.auto_dim_fade_step_ms),
            fade_step_percent: resolved.dimming.auto_dim_fade_step_percent,
            fade_up_step_percent: resolved.dimming.auto_dim_fade_up_step_percent,
            current_brightness: resolved.brightness,
            last_activity: std::time::Instant::now(),
            phase: DimmingPhase::Active,
        };

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<DeviceCommand>();
        let meta_clone = meta.clone();
        let core_context_clone = core_context;
        let serial_clone = serial.clone();
        let device_type_clone = device_type.clone();
        let poll_duration = std::time::Duration::from_millis(config.poll_interval_ms.max(10));

        // Spawn a dedicated thread per device (StreamDeck is !Sync).
        let handle = std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(e) => {
                    error!("StreamDeck service: failed to create tokio runtime for {}: {e}", serial_clone);
                    return;
                }
            };
            runtime.block_on(device_event_loop(
                device,
                rx,
                meta_clone,
                core_context_clone,
                serial_clone,
                device_type_clone,
                key_count,
                key_columns,
                key_width,
                key_height,
                poll_duration,
                dimming,
            ));
        });

        device_threads.push(handle);
        device_entries.push(DeviceEntry {
            serial: serial.clone(),
            device_type: device_type.clone(),
            key_count,
            key_columns,
            key_width,
            key_height,
            sender: tx,
        });
    }

    if device_entries.is_empty() {
        return;
    }

    // Forward incoming commands to device threads.
    // Commands with empty device_id go to all devices; otherwise route by serial.
    while let Some((target_device, command)) = command_receiver.recv().await {
        for entry in &device_entries {
            if target_device.is_empty() || entry.serial == target_device {
                if entry.sender.send(command.clone()).is_err() {
                    error!("StreamDeck service: command channel closed for device {}", entry.serial);
                }
            }
        }
    }

    // Clear all buttons on every device before shutting down.
    for entry in &device_entries {
        if entry.sender.send(DeviceCommand::ClearAllButtons).is_err() {
            error!("StreamDeck service: failed to send ClearAllButtons to device {}", entry.serial);
        }
    }

    // Give device threads time to process the clear command.
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Wait for all device threads to finish.
    for handle in device_threads {
        let _ = handle.join();
    }

    // Broadcast disconnection for all devices.
    for entry in &device_entries {
        broadcast_connection(
            &meta,
            &core_context,
            &entry.serial,
            &entry.device_type,
            entry.key_count,
            entry.key_columns,
            entry.key_width,
            entry.key_height,
            false,
        );
    }

    trace!("StreamDeck service: device loop ended for instance '{}'", meta.id);
}

/// Per-device event loop: reads input, processes commands, and manages dimming.
async fn device_event_loop(
    device: StreamDeck,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<DeviceCommand>,
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
    serial: String,
    device_type: String,
    key_count: u8,
    key_columns: u8,
    key_width: u32,
    key_height: u32,
    poll_duration: std::time::Duration,
    mut dimming: DimmingState,
) {
    let mut previous_states: Vec<bool> = vec![false; key_count as usize];

    loop {
        let dimming_duration = if dimming.enabled {
            dimming.timer_duration()
        } else {
            std::time::Duration::from_secs(86400 * 365)
        };

        tokio::select! {
            command = command_receiver.recv() => {
                match command {
                    Some(DeviceCommand::SetBrightness(percent)) => {
                        dimming.target_brightness = percent;
                        dimming.last_activity = std::time::Instant::now();
                        dimming.phase = DimmingPhase::FadingUp;
                    }
                    Some(DeviceCommand::ClearAllButtons) => {
                        let _ = device.clear_all_button_images();
                        let _ = device.flush();
                    }
                    Some(DeviceCommand::ClearButton(button_index)) => {
                        let _ = device.clear_button_image(button_index);
                        let _ = device.flush();
                    }
                    Some(DeviceCommand::SetButtonImage(button_index, width, height, pixels)) => {
                        let (img_w, img_h) = device.kind().key_image_format().size;
                        let image = render_rgba_to_image(&pixels, width, height, img_w as u32, img_h as u32);
                        let _ = device.set_button_image(button_index, image);
                        let _ = device.flush();
                    }
                    Some(DeviceCommand::Reset) => {
                        let _ = device.reset();
                    }
                    None => break,
                }
            }
            _ = tokio::time::sleep(poll_duration) => {
                match device.read_input(Some(poll_duration)) {
                    Ok(StreamDeckInput::ButtonStateChange(states)) => {
                        for (key, state) in states.iter().enumerate() {
                            if key >= previous_states.len() {
                                continue;
                            }
                            let prev = previous_states[key];
                            let curr = *state;
                            if curr != prev {
                                let msg = MacroPadInputMessage::new(
                                    &serial,
                                    &meta.id,
                                    key as u8,
                                    curr,
                                );
                                broadcast_message(&meta, &core_context, msg);
                                trace!("StreamDeck service: {} key {} {}", serial, key, if curr { "pressed" } else { "released" });
                                previous_states[key] = curr;
                                dimming.last_activity = std::time::Instant::now();
                                if matches!(dimming.phase, DimmingPhase::Dimmed | DimmingPhase::FadingDown) {
                                    dimming.phase = DimmingPhase::FadingUp;
                                }
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        trace!("StreamDeck service: read_input error for {}: {e}", serial);
                    }
                }
            }
            _ = tokio::time::sleep(dimming_duration) => {
                match dimming.phase {
                    DimmingPhase::Active => {
                        if dimming.last_activity.elapsed() >= dimming.idle_timeout {
                            dimming.phase = DimmingPhase::FadingDown;
                        }
                    }
                    DimmingPhase::FadingDown => {
                        dimming.current_brightness = dimming.current_brightness.saturating_sub(dimming.fade_step_percent);
                        if dimming.current_brightness <= dimming.dim_brightness {
                            dimming.current_brightness = dimming.dim_brightness;
                            dimming.phase = DimmingPhase::Dimmed;
                        }
                        if let Err(e) = device.set_brightness(dimming.current_brightness) {
                            error!("StreamDeck service: set_brightness failed for {}: {e}", serial);
                        }
                    }
                    DimmingPhase::Dimmed => {}
                    DimmingPhase::FadingUp => {
                        dimming.current_brightness = dimming.current_brightness.saturating_add(dimming.fade_up_step_percent);
                        if dimming.current_brightness >= dimming.target_brightness {
                            dimming.current_brightness = dimming.target_brightness;
                            dimming.phase = DimmingPhase::Active;
                        }
                        if let Err(e) = device.set_brightness(dimming.current_brightness) {
                            error!("StreamDeck service: set_brightness failed for {}: {e}", serial);
                        }
                    }
                }
            }
        }
    }

    // Broadcast disconnection for this device.
    broadcast_connection(&meta, &core_context, &serial, &device_type, key_count, key_columns, key_width, key_height, false);
}

fn render_rgba_to_image(pixels: &[u8], width: u32, height: u32, target_w: u32, target_h: u32) -> image::DynamicImage {
    use image::Rgba;
    use image::RgbaImage;
    if width == 0 || height == 0 || pixels.len() < (width * height * 4) as usize {
        return image::DynamicImage::ImageRgba8(RgbaImage::from_pixel(target_w, target_h, Rgba([0, 0, 0, 255])));
    }
    let img = RgbaImage::from_raw(width, height, pixels.to_vec()).unwrap_or_else(|| RgbaImage::from_pixel(target_w, target_h, Rgba([0, 0, 0, 255])));
    let resized = image::imageops::resize(&img, target_w, target_h, image::imageops::FilterType::Nearest);
    image::DynamicImage::ImageRgba8(resized)
}

fn broadcast_connection(
    meta: &PluginMeta,
    core_context: &Option<FfiCoreContext>,
    device_id: &str,
    device_type: &str,
    key_count: u8,
    key_columns: u8,
    key_width: u32,
    key_height: u32,
    connected: bool,
) {
    let msg = MacroPadConnectionStatus::new(device_id, &meta.id, device_type, &meta.id, key_count, key_columns, key_width, key_height, connected);
    broadcast_message(meta, core_context, msg);
}

fn broadcast_message<T: Clone + MessageTopic + TypedMessage>(meta: &PluginMeta, core_context: &Option<FfiCoreContext>, message: T) {
    let payload_ptr = Box::into_raw(Box::new(message.clone())) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: meta.id.clone(),
        target_instance_id: stabby::string::String::from(""),
        topic: stabby::string::String::from(T::topic()),
        type_id: T::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(default_destroy_payload),
        clone_payload: Some(default_clone_payload::<T>),
    };
    if let Some(context) = core_context {
        context.send_message(envelope);
    }
}

impl AcceptTopic<FfiEnvelope> for StreamDeckService {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_MACROPAD_COMMAND || topic == TOPIC_MCP_INVOKE_TOOL
    }
}

impl MessageHandler<FfiEnvelopePayload<MacroPadCommandMessage>> for StreamDeckService {
    fn handle_message(&self, message: FfiEnvelopePayload<MacroPadCommandMessage>, _sender_id: &str) {
        let target_device = message.0.device_id.to_string();
        let command = match parse_command(&message.0.command) {
            Some(cmd) => cmd,
            None => {
                debug!("StreamDeck service: failed to parse command");
                return;
            }
        };
        if self.command_sender.send((target_device, command)).is_err() {
            error!("StreamDeck service: command channel closed");
        }
    }
}

fn parse_command(command: &MacroPadCommand) -> Option<DeviceCommand> {
    match command.command_type {
        MacroPadCommandType::SetBrightness => Some(DeviceCommand::SetBrightness(command.percent)),
        MacroPadCommandType::ClearAllButtons => Some(DeviceCommand::ClearAllButtons),
        MacroPadCommandType::ClearButton => Some(DeviceCommand::ClearButton(command.button_index)),
        MacroPadCommandType::SetButtonImage => {
            Some(DeviceCommand::SetButtonImage(command.button_index, command.width, command.height, command.pixels.to_vec()))
        }
        MacroPadCommandType::Reset => Some(DeviceCommand::Reset),
    }
}

impl MessageBroadcaster for StreamDeckService {}

impl PluginMetaGetter for StreamDeckService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for StreamDeckService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl ServicePlugin for StreamDeckService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            let topic = envelope.topic.to_string();
            trace!("StreamDeck service: on_message topic={} type_id={}", topic, envelope.type_id);
            if envelope.type_id == FfiEnvelopePayload::<MacroPadCommandMessage>::TYPE_ID {
                MessageHandler::<FfiEnvelopePayload<MacroPadCommandMessage>>::handle_envelope_message(self, envelope);
            } else if topic == TOPIC_MCP_INVOKE_TOOL && envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
            } else {
                trace!("StreamDeck service: unknown type_id, ignoring");
            }
        }
    }
}
