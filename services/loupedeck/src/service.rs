use crate::config::LoupedeckConfig;
use loupedeck_driver::constants::Event;
use loupedeck_driver::constants::Rgb565;
use loupedeck_driver::devices::Device;
use loupedeck_driver::devices::LoupedeckDevice;
use loupedeck_driver::discovery::Discovery;
use loupedeck_driver::error::LoupedeckError;
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

/// Scale a 0-100 brightness percentage to Loupedeck's 0-10 hardware scale.
/// Uses rounding to avoid the dim brightness dropping to 0 at low percentages.
fn scale_to_loupedeck(percent: u8) -> u8 {
    (((percent as u16 * 10) + 50) / 100) as u8
}

/// Resolved per-device configuration after merging global config with device overrides.
struct ResolvedDeviceConfig {
    brightness: u8,
    dimming: DimmingConfig,
}

/// Resolve per-device configuration by merging global config with device overrides.
fn resolve_device_config(serial: &str, global: &LoupedeckConfig) -> ResolvedDeviceConfig {
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

pub struct LoupedeckService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    #[allow(unused)]
    pub config: LoupedeckConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<(String, DeviceCommand)>,
}

impl LoupedeckService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let service_config = LoupedeckConfig::parse(&config.config)
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
                    error!("Loupedeck service: failed to create tokio runtime: {e}");
                    return;
                }
            };
            runtime.block_on(async move {
                run_device_loop(service_config_clone, command_receiver, meta_clone, core_context_clone).await;
            });
        });

        let service = LoupedeckService {
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
    config: LoupedeckConfig,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<(String, DeviceCommand)>,
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
) {
    debug!("Loupedeck service: starting device loop for instance '{}'", meta.id);

    let devices = match Discovery::available() {
        Ok(devices) => devices,
        Err(e) => {
            error!("Loupedeck service: failed to discover devices: {e}");
            return;
        }
    };

    if devices.is_empty() {
        trace!("Loupedeck service: no Loupedeck devices found");
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

    for device_info in &devices {
        let port_name = device_info.port_name.clone();
        debug!("Loupedeck service: found device at port '{}'", port_name);

        let mut device = match Discovery::connect(device_info.clone()) {
            Ok(device) => device,
            Err(e) => {
                error!("Loupedeck service: failed to connect to device at '{}': {e}", port_name);
                continue;
            }
        };

        let layout = device.layout();
        let device_type = format!("{:?}", device_info.device);
        let key_count = (layout.columns * layout.rows) as u8;
        let key_columns = layout.columns as u8;
        let key_width = layout.key_size as u32;
        let key_height = layout.key_size as u32;

        // Get serial number from device.
        let serial = match device.get_serial() {
            Ok(Some(bytes)) => String::from_utf8_lossy(&bytes).to_string(),
            _ => port_name.clone(),
        };

        let resolved = resolve_device_config(&serial, &config);

        let hardware_brightness = scale_to_loupedeck(resolved.brightness);
        if let Err(e) = device.set_brightness(hardware_brightness) {
            error!("Loupedeck service: failed to set brightness for {}: {e}", serial);
        }

        // Broadcast connection status for this device.
        broadcast_connection(&meta, &core_context, &serial, &device_type, key_count, key_columns, key_width, key_height, true);

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

        let handle = std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(e) => {
                    error!("Loupedeck service: failed to create tokio runtime for {}: {e}", serial_clone);
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
            serial,
            device_type,
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
                    error!("Loupedeck service: command channel closed for device {}", entry.serial);
                }
            }
        }
    }

    // Clear all buttons on every device before shutting down.
    for entry in &device_entries {
        if entry.sender.send(DeviceCommand::ClearAllButtons).is_err() {
            error!("Loupedeck service: failed to send ClearAllButtons to device {}", entry.serial);
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

    trace!("Loupedeck service: device loop ended for instance '{}'", meta.id);
}

async fn device_event_loop(
    mut device: Device,
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
    let mut button_states: Vec<bool> = vec![false; key_count as usize];

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
                        // Clear by drawing black to all key positions.
                        let black = Rgb565::from_rgb8(0, 0, 0);
                        let layout = device.layout();
                        let key_size = layout.key_size as u16;
                        let key_stride = layout.key_stride as u16;
                        let offset_x = (key_stride - key_size) / 2;
                        let offset_y: u16 = 6;
                        for col in 0..layout.columns {
                            for row in 0..layout.rows {
                                let x = col as u16 * key_stride + offset_x;
                                let y = row as u16 * key_stride + offset_y;
                                let pixels = vec![black; (key_size as usize) * (key_size as usize)];
                                if let Err(e) = device.draw(x, y, key_size, key_size, &pixels) {
                                    error!("Loupedeck service: draw failed for clear on {}: {e}", serial);
                                }
                            }
                        }
                        if let Err(e) = device.refresh() {
                            error!("Loupedeck service: refresh failed for clear on {}: {e}", serial);
                        }
                    }
                    Some(DeviceCommand::ClearButton(button_index)) => {
                        let layout = device.layout();
                        let cols = layout.columns as u8;
                        if button_index < key_count {
                            let col = button_index % cols;
                            let row = button_index / cols;
                            let key_size = layout.key_size as u16;
                            let key_stride = layout.key_stride as u16;
                            let offset_x = (key_stride - key_size) / 2;
                            let offset_y: u16 = 6;
                            let x = col as u16 * key_stride + offset_x;
                            let y = row as u16 * key_stride + offset_y;
                            let black = Rgb565::from_rgb8(0, 0, 0);
                            let pixels = vec![black; (key_size as usize) * (key_size as usize)];
                            if let Err(e) = device.draw(x, y, key_size, key_size, &pixels) {
                                error!("Loupedeck service: draw failed for clear button {} on {}: {e}", button_index, serial);
                            }
                            if let Err(e) = device.refresh() {
                                error!("Loupedeck service: refresh failed for clear button {} on {}: {e}", button_index, serial);
                            }
                        }
                    }
                    Some(DeviceCommand::SetButtonImage(button_index, width, height, pixels)) => {
                        let layout = device.layout();
                        let cols = layout.columns as u8;
                        let key_size = layout.key_size as u16;
                        let key_stride = layout.key_stride as u16;
                        let offset_x = (key_stride - key_size) / 2;
                        let offset_y: u16 = 6;
                        if button_index < key_count {
                            let col = button_index % cols;
                            let row = button_index / cols;
                            let x = col as u16 * key_stride + offset_x;
                            let y = row as u16 * key_stride + offset_y;
                            let rgb565_pixels = render_rgba_to_rgb565(&pixels, width, height, key_size as u32, key_size as u32);
                            if let Err(e) = device.draw(x, y, key_size, key_size, &rgb565_pixels) {
                                error!("Loupedeck service: draw failed for button {} on {}: {e}", button_index, serial);
                            }
                            if let Err(e) = device.refresh() {
                                error!("Loupedeck service: refresh failed for button {} on {}: {e}", button_index, serial);
                            }
                        }
                    }
                    Some(DeviceCommand::Reset) => {
                        let _ = device.reset();
                    }
                    None => break,
                }
            }
            _ = tokio::time::sleep(poll_duration) => {
                match device.get_evt() {
                    Ok(Some(event)) => {
                        let was_button_press = matches!(event, Event::ButtonPress { .. });
                        handle_event(&event, &serial, &meta, &core_context, &mut button_states);
                        if was_button_press {
                            dimming.last_activity = std::time::Instant::now();
                            if matches!(dimming.phase, DimmingPhase::Dimmed | DimmingPhase::FadingDown) {
                                dimming.phase = DimmingPhase::FadingUp;
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        if !is_serial_timeout(&e) {
                            trace!("Loupedeck service: get_evt error for {}: {e}", serial);
                        }
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
                        let hardware_brightness = scale_to_loupedeck(dimming.current_brightness);
                        if let Err(e) = device.set_brightness(hardware_brightness) {
                            error!("Loupedeck service: set_brightness failed for {}: {e}", serial);
                        }
                    }
                    DimmingPhase::Dimmed => {}
                    DimmingPhase::FadingUp => {
                        dimming.current_brightness = dimming.current_brightness.saturating_add(dimming.fade_up_step_percent);
                        if dimming.current_brightness >= dimming.target_brightness {
                            dimming.current_brightness = dimming.target_brightness;
                            dimming.phase = DimmingPhase::Active;
                        }
                        let hardware_brightness = scale_to_loupedeck(dimming.current_brightness);
                        if let Err(e) = device.set_brightness(hardware_brightness) {
                            error!("Loupedeck service: set_brightness failed for {}: {e}", serial);
                        }
                    }
                }
            }
        }
    }

    // Broadcast disconnection for this device.
    broadcast_connection(&meta, &core_context, &serial, &device_type, key_count, key_columns, key_width, key_height, false);
}

fn is_serial_timeout(e: &LoupedeckError) -> bool {
    matches!(e, LoupedeckError::Io(io_err) if io_err.kind() == std::io::ErrorKind::TimedOut)
}

fn handle_event(event: &Event, serial: &str, meta: &PluginMeta, core_context: &Option<FfiCoreContext>, button_states: &mut [bool]) {
    match event {
        Event::ButtonPress { button_id, press } => {
            let index = *button_id as usize;
            let pressed = *press == 0;
            if index < button_states.len() {
                button_states[index] = pressed;
            }
            let msg = MacroPadInputMessage::new(serial, &meta.id, *button_id, pressed);
            broadcast_message(meta, core_context, msg);
            trace!("Loupedeck service: {} button {} {}", serial, button_id, if pressed { "pressed" } else { "released" });
        }
        Event::KnobRotate { knob_id, delta } => {
            trace!("Loupedeck service: {} knob {} rotated by {}", serial, knob_id, delta);
        }
        Event::Touch { x, y, id } => {
            trace!("Loupedeck service: {} touch at ({}, {}) id={}", serial, x, y, id);
        }
        Event::TouchRelease { x, y, id } => {
            trace!("Loupedeck service: {} touch release at ({}, {}) id={}", serial, x, y, id);
        }
        Event::Raw { command_id, data } => {
            let hex: Vec<String> = data.iter().map(|b| format!("0x{:02x}", b)).collect();
            trace!(
                "Loupedeck service: {} raw event: command_id=0x{:02x} {} bytes: [{}]",
                serial,
                command_id,
                data.len(),
                hex.join(", ")
            );
        }
    }
}

fn render_rgba_to_rgb565(pixels: &[u8], width: u32, height: u32, target_w: u32, target_h: u32) -> Vec<Rgb565> {
    use image::Rgba;
    use image::RgbaImage;
    if width == 0 || height == 0 || pixels.len() < (width * height * 4) as usize {
        let black = Rgb565::from_rgb8(0, 0, 0);
        return vec![black; (target_w as usize) * (target_h as usize)];
    }
    let img = RgbaImage::from_raw(width, height, pixels.to_vec()).unwrap_or_else(|| RgbaImage::from_pixel(target_w, target_h, Rgba([0, 0, 0, 255])));
    let resized = image::imageops::resize(&img, target_w, target_h, image::imageops::FilterType::Nearest);
    resized.pixels().map(|p| Rgb565::from_rgb8(p[0], p[1], p[2])).collect()
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

impl AcceptTopic<FfiEnvelope> for LoupedeckService {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_MACROPAD_COMMAND || topic == TOPIC_MCP_INVOKE_TOOL
    }
}

impl MessageHandler<FfiEnvelopePayload<MacroPadCommandMessage>> for LoupedeckService {
    fn handle_message(&self, message: FfiEnvelopePayload<MacroPadCommandMessage>, _sender_id: &str) {
        let target_device = message.0.device_id.to_string();
        let command = match parse_command(&message.0.command) {
            Some(cmd) => cmd,
            None => {
                trace!("Loupedeck service: failed to parse command");
                return;
            }
        };
        if self.command_sender.send((target_device, command)).is_err() {
            error!("Loupedeck service: command channel closed");
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

impl MessageBroadcaster for LoupedeckService {}

impl PluginMetaGetter for LoupedeckService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for LoupedeckService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl ServicePlugin for LoupedeckService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            let topic = envelope.topic.to_string();
            trace!("Loupedeck service: on_message topic={} type_id={}", topic, envelope.type_id);
            if envelope.type_id == FfiEnvelopePayload::<MacroPadCommandMessage>::TYPE_ID {
                MessageHandler::<FfiEnvelopePayload<MacroPadCommandMessage>>::handle_envelope_message(self, envelope);
            } else if topic == TOPIC_MCP_INVOKE_TOOL && envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
            } else {
                trace!("Loupedeck service: unknown type_id, ignoring");
            }
        }
    }
}
