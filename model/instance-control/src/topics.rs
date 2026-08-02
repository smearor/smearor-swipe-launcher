/// Topic to dynamically load a new launcher instance.
pub const TOPIC_CORE_INSTANCE_LOAD: &str = "core.instance.load";
/// Topic to dynamically stop a running launcher instance.
pub const TOPIC_CORE_INSTANCE_STOP: &str = "core.instance.stop";
/// Topic to hot-reload a running instance (stop + load with same ID).
pub const TOPIC_CORE_INSTANCE_RELOAD: &str = "core.instance.reload";
/// Topic for instance status broadcasts (Loaded / Stopped / Reloaded).
pub const TOPIC_CORE_INSTANCE_STATUS: &str = "core.instance.status";
