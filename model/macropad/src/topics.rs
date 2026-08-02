/// Topics for MacroPad cross-instance messaging.
///
/// These topics are used by MacroPad services (Stream Deck, Loupedeck) to
/// communicate with the host and other instances via the message broker.

/// Topic for input events from MacroPad devices (button presses, releases, encoder turns).
pub const TOPIC_MACROPAD_INPUT: &str = "service.macropad.input";

/// Topic for connection status updates from MacroPad services.
pub const TOPIC_MACROPAD_CONNECTION: &str = "service.macropad.connection";

/// Topic for commands sent to MacroPad services (set brightness, clear button, set image).
pub const TOPIC_MACROPAD_COMMAND: &str = "service.macropad.command";
