/// Action to perform when a message is received on an `auto_start_topic` or `auto_stop_topic`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopicAction {
    /// Start the associated instance.
    Start,
    /// Stop the associated instance.
    Stop,
}
