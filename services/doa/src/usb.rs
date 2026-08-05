use crate::config::DoaServiceConfig;
use rusb::Context;
use rusb::DeviceHandle;
use rusb::UsbContext;
use std::time::Duration;
use tracing::debug;
use tracing::error;
use tracing::trace;
use tracing::warn;

/// USB Vendor IDs for ReSpeaker / XMOS devices.
const VENDOR_ID_SEEED: u16 = 0x2886;
const VENDOR_ID_XMOS: u16 = 0x20b1;

/// USB Control Transfer parameters for XVF3800.
const REQUEST_TYPE_READ: u8 = 0xC0;
const B_REQUEST_READ: u8 = 0x00;
const PARAM_DOA_ANGLE: u16 = 0x0015;
const PARAM_VAD: u16 = 0x0016;

/// Result of a single DoA USB read, sent from the USB reader thread to the async loop.
pub enum DoaReading {
    /// Successful DoA read with angle and speech activity flag.
    Reading {
        angle: u16,
        speech_detected: bool,
        vendor_id: u16,
        product_id: u16,
    },
    /// USB read failed or device was lost.
    Disconnected,
}

/// Control commands forwarded from the async loop to the USB reader thread.
pub enum UsbControl {
    /// Pause polling (stop reading from the device, keep handle open).
    Pause,
    /// Resume polling.
    Resume,
    /// Close current handle and attempt reconnection.
    Reconnect,
    /// Change the polling interval.
    SetInterval(u64),
}

/// Searches for a ReSpeaker/XMOS USB device and opens a DeviceHandle.
/// The `old_handle` parameter ensures the previous handle is dropped before
/// opening a new one, releasing USB interface claims and letting libusb clean up.
pub fn open_respeaker(config: &DoaServiceConfig, old_handle: Option<DeviceHandle<Context>>) -> Option<DeviceHandle<Context>> {
    drop(old_handle);
    let context = Context::new().ok()?;
    for device in context.devices().ok()?.iter() {
        let device_desc = device.device_descriptor().ok()?;
        let vid = device_desc.vendor_id();
        if vid == VENDOR_ID_SEEED || vid == VENDOR_ID_XMOS {
            if let Some(pid) = config.product_id {
                if device_desc.product_id() != pid {
                    continue;
                }
            }
            if let Ok(handle) = device.open() {
                debug!("DoA service: connected to USB device VID={:#06x} PID={:#06x}", vid, device_desc.product_id());
                return Some(handle);
            }
        }
    }
    trace!("DoA service: no ReSpeaker XVF3800 USB device found");
    None
}

/// Reads the DoA angle (0-359 degrees) via a USB Control Transfer.
/// The timeout is derived from the current poll interval to ensure that
/// a single stalled transfer cannot block the USB thread longer than
/// one poll cycle. See `usb_transfer_timeout`.
pub fn read_doa_angle(handle: &DeviceHandle<Context>, timeout: Duration) -> Result<u16, rusb::Error> {
    let mut buffer = [0u8; 8];
    let bytes_read = handle.read_control(REQUEST_TYPE_READ, B_REQUEST_READ, PARAM_DOA_ANGLE, 0x0000, &mut buffer, timeout)?;
    if bytes_read >= 2 {
        let raw_angle = u16::from_le_bytes([buffer[0], buffer[1]]);
        Ok(raw_angle % 360)
    } else {
        Err(rusb::Error::InvalidParam)
    }
}

/// Reads the Voice Activity Detection (VAD) flag via a USB Control Transfer.
/// Returns `true` when the DSP detects active speech, `false` during silence.
/// When `false`, the DoA angle register holds the last detected direction.
/// The timeout is derived from the current poll interval (see `usb_transfer_timeout`).
pub fn read_speech_detected(handle: &DeviceHandle<Context>, timeout: Duration) -> Result<bool, rusb::Error> {
    let mut buffer = [0u8; 8];
    let bytes_read = handle.read_control(REQUEST_TYPE_READ, B_REQUEST_READ, PARAM_VAD, 0x0000, &mut buffer, timeout)?;
    if bytes_read >= 1 {
        Ok(buffer[0] != 0)
    } else {
        Err(rusb::Error::InvalidParam)
    }
}

/// Extracts vendor ID and product ID from a DeviceHandle's device descriptor.
pub fn device_vid_pid(handle: &DeviceHandle<Context>) -> (u16, u16) {
    match handle.device().device_descriptor() {
        Ok(desc) => (desc.vendor_id(), desc.product_id()),
        Err(_) => (0, 0),
    }
}

/// Computes the USB Control Transfer timeout from the current poll interval.
/// Set to half the poll interval, clamped to [20, 100] ms, so that two
/// consecutive stalled transfers cannot block the USB thread longer than
/// one poll cycle — even when `poll_interval_ms` is reduced to the 50 ms minimum.
pub fn usb_transfer_timeout(poll_interval_ms: u64) -> Duration {
    Duration::from_millis((poll_interval_ms / 2).clamp(20, 100))
}

/// Produces an initial DoaReading reflecting the current connection state.
pub fn initial_reading(handle: &Option<DeviceHandle<Context>>) -> DoaReading {
    match handle {
        Some(h) => {
            let (vid, pid) = device_vid_pid(h);
            DoaReading::Reading {
                angle: 0,
                speech_detected: false,
                vendor_id: vid,
                product_id: pid,
            }
        }
        None => DoaReading::Disconnected,
    }
}

/// Classifies a USB error and handles reconnection with appropriate logging.
///
/// Error categories:
/// - **Physical disconnect** (`NoDevice`, `NotFound`, `Io`): device is gone.
///   Drop the handle explicitly, log once at `warn!`, then reconnect.
/// - **Transient** (`Busy`, `Timeout`, `Pipe`): keep the handle open, back off
///   with exponential delay, and suppress repeated log messages to avoid spam.
/// - **Unexpected**: log at `error!`, drop handle, and attempt full reconnection.
pub fn classify_and_handle_usb_error(
    error: &rusb::Error,
    consecutive_failures: &mut u32,
    config: &DoaServiceConfig,
    handle: &mut Option<DeviceHandle<Context>>,
    reading_sender: &tokio::sync::mpsc::UnboundedSender<DoaReading>,
) {
    *consecutive_failures += 1;
    match error {
        rusb::Error::NoDevice | rusb::Error::NotFound | rusb::Error::Io => {
            warn!("DoA USB thread: device disconnected ({:?}), dropping handle", error);
            let old = handle.take();
            *handle = open_respeaker(config, old);
            let _ = reading_sender.send(initial_reading(handle));
            if handle.is_some() {
                *consecutive_failures = 0;
            } else {
                std::thread::sleep(Duration::from_millis(config.reconnect_delay_ms));
            }
        }
        rusb::Error::Busy | rusb::Error::Timeout | rusb::Error::Pipe => {
            let backoff_ms = config.reconnect_delay_ms.saturating_mul((*consecutive_failures).min(10) as u64);
            if *consecutive_failures <= 3 || *consecutive_failures % 10 == 0 {
                debug!(
                    "DoA USB thread: transient error ({:?}), retrying after {}ms (attempt {})",
                    error, backoff_ms, consecutive_failures
                );
            }
            std::thread::sleep(Duration::from_millis(backoff_ms));
        }
        other => {
            error!("DoA USB thread: unexpected USB error ({:?}), reconnecting", other);
            let old = handle.take();
            *handle = open_respeaker(config, old);
            let _ = reading_sender.send(initial_reading(handle));
            if handle.is_some() {
                *consecutive_failures = 0;
            } else {
                std::thread::sleep(Duration::from_millis(config.reconnect_delay_ms));
            }
        }
    }
}

/// Runs on a dedicated OS thread. Owns the USB DeviceHandle and performs blocking reads.
pub fn usb_reader_loop(
    config: DoaServiceConfig,
    reading_sender: tokio::sync::mpsc::UnboundedSender<DoaReading>,
    mut control_receiver: tokio::sync::mpsc::UnboundedReceiver<UsbControl>,
) {
    let mut handle = open_respeaker(&config, None);
    let mut paused = false;
    let mut poll_interval_ms = config.poll_interval_ms;
    let mut consecutive_failures: u32 = 0;

    let _ = reading_sender.send(initial_reading(&handle));

    loop {
        match control_receiver.blocking_recv() {
            Some(UsbControl::Pause) => {
                paused = true;
                debug!("DoA USB thread: paused");
            }
            Some(UsbControl::Resume) => {
                paused = false;
                debug!("DoA USB thread: resumed");
            }
            Some(UsbControl::SetInterval(ms)) => {
                poll_interval_ms = ms.max(50);
                debug!("DoA USB thread: interval set to {}ms", poll_interval_ms);
            }
            Some(UsbControl::Reconnect) => {
                debug!("DoA USB thread: reconnecting (manual)...");
                let old_handle = handle.take();
                handle = open_respeaker(&config, old_handle);
                consecutive_failures = 0;
                let _ = reading_sender.send(initial_reading(&handle));
            }
            None => {
                debug!("DoA USB thread: control channel closed, exiting");
                drop(handle.take());
                return;
            }
        }

        if paused {
            continue;
        }

        match &handle {
            Some(device_handle) => {
                let transfer_timeout = usb_transfer_timeout(poll_interval_ms);
                match read_doa_angle(device_handle, transfer_timeout) {
                    Ok(angle) => {
                        let speech_detected = read_speech_detected(device_handle, transfer_timeout).unwrap_or_else(|e| {
                            debug!("DoA USB thread: VAD read failed ({:?}), falling back to false", e);
                            false
                        });
                        let (vid, pid) = device_vid_pid(device_handle);
                        let _ = reading_sender.send(DoaReading::Reading {
                            angle,
                            speech_detected,
                            vendor_id: vid,
                            product_id: pid,
                        });
                        consecutive_failures = 0;
                    }
                    Err(e) => {
                        classify_and_handle_usb_error(&e, &mut consecutive_failures, &config, &mut handle, &reading_sender);
                    }
                }
            }
            None => {
                std::thread::sleep(Duration::from_millis(config.reconnect_delay_ms));
                handle = open_respeaker(&config, None);
                if handle.is_some() {
                    consecutive_failures = 0;
                }
                let _ = reading_sender.send(initial_reading(&handle));
            }
        }
    }
}
