/// Generates a deterministic `u64` hash from a string.
///
/// This uses the FNV-1a hash algorithm, which is simple, fast, and
/// produces consistent results across compilations and crates.
///
/// # When to Use
///
/// Use this function (at compile time via `const`) to generate stable
/// `type_id` values for message types. **Do not** use
/// `std::any::TypeId::of::<T>()` — it is not stable across crate
/// boundaries or compiler versions.
///
/// # Example
///
/// ```rust
/// pub const MY_MESSAGE_TYPE_ID: u64 = generate_type_id("model::notifications::MyMessage");
/// ```
pub const fn generate_type_id(name: &str) -> u64 {
    let bytes = name.as_bytes();
    let mut hash = 0xcbf29ce484222325u64;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        index += 1;
    }
    hash
}

/// Marker trait for types that have a stable, deterministic `type_id`.
///
/// Message structs in `model` crates implement this trait to
/// advertise their unique type identifier for cross-plugin down-casting.
pub trait TypedMessage {
    /// A stable type identifier for this message type.
    ///
    /// This must be generated via `generate_type_id` with the fully
    /// qualified type path as the input string.
    const TYPE_ID: u64;
}
