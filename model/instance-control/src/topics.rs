/// Topic to dynamically load a new launcher instance.
pub const TOPIC_CORE_INSTANCE_LOAD: &str = "core.instance.load";
/// Topic to dynamically start a loaded (Ready) launcher instance.
pub const TOPIC_CORE_INSTANCE_START: &str = "core.instance.start";
/// Topic to dynamically stop a running launcher instance.
pub const TOPIC_CORE_INSTANCE_STOP: &str = "core.instance.stop";
/// Topic to dynamically unload a stopped (Ready) launcher instance.
pub const TOPIC_CORE_INSTANCE_UNLOAD: &str = "core.instance.unload";
/// Topic to hot-reload a running instance (stop + load with same ID).
pub const TOPIC_CORE_INSTANCE_RELOAD: &str = "core.instance.reload";
/// Topic for instance status broadcasts (Loaded / Started / Stopped / Unloaded / Reloaded).
pub const TOPIC_CORE_INSTANCE_STATUS: &str = "core.instance.status";
