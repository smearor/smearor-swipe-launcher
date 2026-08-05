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
/// The XVF3800 uses a resource/command ID protocol:
/// - wValue = 0x80 | cmdid (high bit set for reads)
/// - wIndex = resid (resource ID)
/// - Response: byte 0 = status, followed by payload data
const REQUEST_TYPE_READ: u8 = 0xC0;
const B_REQUEST_READ: u8 = 0x00;
const CMDID_READ_FLAG: u8 = 0x80;
const RESID_DOA: u16 = 20;
const CMDID_DOA: u8 = 18;
const DOA_RESPONSE_LEN: usize = 5;

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
            match device.open() {
                Ok(handle) => {
                    debug!("DoA service: connected to USB device VID={:#06x} PID={:#06x}", vid, device_desc.product_id());
                    return Some(handle);
                }
                Err(e) => {
                    warn!(
                        "DoA service: found USB device VID={:#06x} PID={:#06x} but failed to open: {:?}",
                        vid,
                        device_desc.product_id(),
                        e
                    );
                }
            }
        }
    }
    trace!("DoA service: no ReSpeaker XVF3800 USB device found");
    None
}

/// Reads the DoA angle (0-359 degrees) and VAD flag in a single USB Control Transfer.
/// The XVF3800 returns both values in one response: status byte, angle (uint16 LE), VAD (uint16 LE).
/// The timeout is derived from the current poll interval to ensure that
/// a single stalled transfer cannot block the USB thread longer than
/// one poll cycle. See `usb_transfer_timeout`.
pub fn read_doa(handle: &DeviceHandle<Context>, timeout: Duration) -> Result<(u16, bool), rusb::Error> {
    let mut buffer = [0u8; DOA_RESPONSE_LEN];
    let w_value = u16::from(CMDID_READ_FLAG | CMDID_DOA);
    let bytes_read = handle.read_control(REQUEST_TYPE_READ, B_REQUEST_READ, w_value, RESID_DOA, &mut buffer, timeout)?;
    if bytes_read >= DOA_RESPONSE_LEN {
        let angle = u16::from_le_bytes([buffer[1], buffer[2]]);
        let vad = u16::from_le_bytes([buffer[3], buffer[4]]) != 0;
        Ok((angle % 360, vad))
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
        while let Ok(cmd) = control_receiver.try_recv() {
            match cmd {
                UsbControl::Pause => {
                    paused = true;
                    debug!("DoA USB thread: paused");
                }
                UsbControl::Resume => {
                    paused = false;
                    debug!("DoA USB thread: resumed");
                }
                UsbControl::SetInterval(ms) => {
                    poll_interval_ms = ms.max(50);
                    debug!("DoA USB thread: interval set to {}ms", poll_interval_ms);
                }
                UsbControl::Reconnect => {
                    debug!("DoA USB thread: reconnecting (manual)...");
                    let old_handle = handle.take();
                    handle = open_respeaker(&config, old_handle);
                    consecutive_failures = 0;
                    let _ = reading_sender.send(initial_reading(&handle));
                }
            }
        }
        if control_receiver.is_closed() {
            debug!("DoA USB thread: control channel closed, exiting");
            drop(handle.take());
            return;
        }

        let cycle_start = std::time::Instant::now();

        if paused {
            let elapsed = cycle_start.elapsed();
            let remaining = Duration::from_millis(poll_interval_ms).saturating_sub(elapsed);
            std::thread::sleep(remaining);
            continue;
        }

        match &handle {
            Some(device_handle) => {
                let transfer_timeout = usb_transfer_timeout(poll_interval_ms);
                match read_doa(device_handle, transfer_timeout) {
                    Ok((angle, speech_detected)) => {
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
                handle = open_respeaker(&config, None);
                if handle.is_some() {
                    consecutive_failures = 0;
                    let _ = reading_sender.send(initial_reading(&handle));
                } else {
                    std::thread::sleep(Duration::from_millis(config.reconnect_delay_ms));
                }
            }
        }

        let elapsed = cycle_start.elapsed();
        let remaining = Duration::from_millis(poll_interval_ms).saturating_sub(elapsed);
        std::thread::sleep(remaining);
    }
}
