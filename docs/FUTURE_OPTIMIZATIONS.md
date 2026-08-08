# Future Optimizations

## 1. `clone_payload` → `Arc<T>` for Service Broadcast Payloads

### Problem

When a service broadcasts a typed message (e.g. `service.audio.status`) to multiple launcher instances, the current `FfiEnvelope::clone()` implementation calls
`clone_payload` for every receiver. This allocates a new heap block per instance:

- 2 instances → 2 `Box::new(status.clone())` calls
- 4 instances → 4 `Box::new(status.clone())` calls
- 10 instances → 10 `Box::new(status.clone())` calls

Each payload is de-allocated independently by `destroy_payload`.

For a Smart Desk with 4 sides, this is negligible (~4 heap ops/sec per service). However, if the number of instances grows significantly (>10), the cumulative
allocation overhead becomes measurable.

### Proposed Solution

Replace per-instance heap cloning with reference-counted sharing using `Arc<T>`:

1. **Service sends** an `Arc<T>` wrapped in a thin FFI-compatible pointer.
2. **Host stores** the `Arc<T>` internally.
3. **Each instance receives** a borrowed pointer (or a cloned `Arc`).
4. **Last instance calls** `destroy_payload`, which decrements the `Arc` reference count. The heap block is freed only when the last reference drops.

### Implementation Sketch

```rust
// plugin-api/src/messages.rs
pub struct FfiEnvelope {
    // ... existing fields ...
    pub payload: *mut core::ffi::c_void,
    pub destroy_payload: Option<extern "C" fn(*mut core::ffi::c_void)>,
    pub clone_payload: Option<extern "C" fn(*mut core::ffi::c_void) -> *mut core::ffi::c_void>,
}

// Future: add a reference-counted payload mode
pub struct SharedPayload<T> {
    arc: Arc<T>,
}

impl<T: Clone + Send + Sync + 'static> SharedPayload<T> {
    pub fn new(value: T) -> Self {
        Self { arc: Arc::new(value) }
    }

    pub fn as_ptr(&self) -> *mut core::ffi::c_void {
        Arc::into_raw(self.arc.clone()) as *mut core::ffi::c_void
    }

    pub unsafe fn from_ptr(ptr: *mut core::ffi::c_void) -> Arc<T> {
        Arc::from_raw(ptr as *const T)
    }
}
```

### Service-side Change (example: audio service)

```rust
// Instead of:
let payload_ptr = Box::into_raw(Box::new(status)) as * mut c_void;

// Future:
let shared = SharedPayload::new(status);
let payload_ptr = shared.as_ptr();
// store `shared` in service state so it lives until shutdown
```

### Host-side Change

In `LauncherHost::route_message`, when broadcasting `service.*` topics:

```rust
// Instead of:
for instance in instances.values() {
instance.handle_message(envelope.clone()); // clones payload each time
}

// Future:
for instance in instances.values() {
// Borrow the same Arc without cloning the inner data
let borrowed = envelope.with_borrowed_payload();
instance.handle_message(borrowed);
}
```

### Trade-offs

| Aspect                         | Current (`clone_payload`) | Future (`Arc<T>`)                   |
|--------------------------------|---------------------------|-------------------------------------|
| Heap allocations per broadcast | N instances               | 1 (shared)                          |
| Deallocations                  | N independent             | 1 (last drop)                       |
| CPU cost per clone             | `T::clone()` (deep copy)  | `Arc::clone()` (atomic inc)         |
| Memory footprint               | N × sizeof(T)             | 1 × sizeof(T) + refcount            |
| Complexity                     | Low                       | Medium (needs FFI-safe Arc wrapper) |
| Thread safety                  | N/A                       | Requires `Send + Sync` on `T`       |

### When to Implement

- **< 5 instances**: Not worth it. Current overhead is < 1 ms/day.
- **5–10 instances**: Optional. Measurable but not user-visible.
- **> 10 instances** (e.g. multi-seat kiosk, classroom panels): Recommended. Allocation pressure becomes significant at high broadcast frequencies.

### Related Files

- `plugin-api/src/messages.rs` — `FfiEnvelope`, `clone_payload`
- `services/audio/src/service/loaded_service.rs` — `clone_audio_status`
- `services/mpris/src/service/loaded_service.rs` — `clone_mpris_status`
- `services/notifications/src/service/loaded_service.rs` — `clone_notification_status`
- `smearor-swipe-launcher/src/host/mod.rs` — `route_message` broadcast loop
