use crate::SmearorWindowRotationWrapper;
use crate::SmearorWindowRotationWrapperStabby;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_COMMAND: &str = "service.app_launcher.command";

#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub enum DesktopFileCommandAction {
    #[default]
    Exec,
    ExecStart,
    ExecReload,
    Terminate,
}

#[derive(Clone, Debug, Default)]
pub struct DesktopFileCommandMessage {
    /// The canonical path of the desktop file
    pub desktop_file: String,

    /// The smearor window rotation wrapper configuration
    pub wrapper: Option<SmearorWindowRotationWrapper>,

    /// The action to execute on the desktop file
    pub action: DesktopFileCommandAction,
}

/// ABI-stable version of `DesktopFileCommandMessage` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct DesktopFileCommandMessageStabby {
    pub desktop_file: stabby::string::String,
    pub wrapper: stabby::option::Option<SmearorWindowRotationWrapperStabby>,
    pub action: DesktopFileCommandAction,
}

impl From<DesktopFileCommandMessage> for DesktopFileCommandMessageStabby {
    fn from(value: DesktopFileCommandMessage) -> Self {
        Self {
            desktop_file: value.desktop_file.into(),
            wrapper: value.wrapper.map(Into::into).into(),
            action: value.action,
        }
    }
}

impl From<DesktopFileCommandMessageStabby> for DesktopFileCommandMessage {
    fn from(value: DesktopFileCommandMessageStabby) -> Self {
        Self {
            desktop_file: value.desktop_file.to_string(),
            wrapper: {
                let opt: Option<SmearorWindowRotationWrapperStabby> = value.wrapper.into();
                opt.map(Into::into)
            },
            action: value.action,
        }
    }
}

impl DesktopFileCommandMessage {
    pub fn new(desktop_file: &str, wrapper: Option<SmearorWindowRotationWrapper>, action: DesktopFileCommandAction) -> Self {
        Self {
            desktop_file: desktop_file.to_string(),
            wrapper,
            action,
        }
    }

    pub fn exec(desktop_file: &str, wrapper: Option<SmearorWindowRotationWrapper>) -> Self {
        Self::new(desktop_file, wrapper, DesktopFileCommandAction::Exec)
    }

    pub fn exec_start(desktop_file: &str, wrapper: Option<SmearorWindowRotationWrapper>) -> Self {
        Self::new(desktop_file, wrapper, DesktopFileCommandAction::ExecStart)
    }

    pub fn terminate(desktop_file: &str, wrapper: Option<SmearorWindowRotationWrapper>) -> Self {
        Self::new(desktop_file, wrapper, DesktopFileCommandAction::Terminate)
    }
}

impl TypedMessage for DesktopFileCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_app_launcher_model::DesktopFileCommandMessageStabby");
}

impl TypedMessage for DesktopFileCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_app_launcher_model::DesktopFileCommandMessage");
}

impl MessageTopic for DesktopFileCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl MessageTopic for DesktopFileCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for DesktopFileCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
