/// FFI callback type for forwarding log events from plugin to host.
///
/// The host implements this callback to push `LogEntry` records into its
/// `LogBuffer`.  All string pointers are borrowed for the duration of the
/// call only — the host must copy the data if it needs to retain it.
///
/// # Safety
///
/// The host guarantees `context` remains valid for the plugin's lifetime.
/// All `(ptr, len)` pairs must point to valid UTF-8 bytes.
pub type LogForwardFn = unsafe extern "C" fn(
    context: *const core::ffi::c_void,
    level: u8,
    target_ptr: *const u8,
    target_len: usize,
    message_ptr: *const u8,
    message_len: usize,
    file_ptr: *const u8,
    file_len: usize,
    line: u32,
    timestamp_ms: u64,
);

/// Handle holding the host's log-forward callback and opaque context pointer.
///
/// Embedded in `FfiCoreContext`.  When `Some`, plugins install a
/// `LogForwardLayer` that forwards all `tracing` events to the host.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LogForwardHandle {
    /// Opaque pointer to the host's `SimpleCoreContext` (same pointer as
    /// `broker.context`).
    pub context: *const core::ffi::c_void,
    /// The host's log-forward callback function.
    pub forward: LogForwardFn,
}

impl std::fmt::Debug for LogForwardHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogForwardHandle")
            .field("context", &self.context)
            .field("forward", &"<fn>")
            .finish()
    }
}

// SAFETY: The host guarantees the callback is thread-safe and the context
// pointer remains valid for the plugin's lifetime.
unsafe impl Send for LogForwardHandle {}
unsafe impl Sync for LogForwardHandle {}

/// No-op log forward function used as placeholder when core_context is null.
pub unsafe extern "C" fn dummy_log_forward(
    _context: *const core::ffi::c_void,
    _level: u8,
    _target_ptr: *const u8,
    _target_len: usize,
    _message_ptr: *const u8,
    _message_len: usize,
    _file_ptr: *const u8,
    _file_len: usize,
    _line: u32,
    _timestamp_ms: u64,
) {
}
