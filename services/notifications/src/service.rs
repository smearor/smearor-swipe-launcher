use crate::config::NotificationServiceConfig;
use smearor_notifications_model::NotificationCommandAction;
use smearor_notifications_model::NotificationCommandMessage;
use smearor_notifications_model::NotificationInfo;
use smearor_notifications_model::NotificationStatusMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tracing::debug;
use tracing::info;

#[derive(Debug)]
pub enum NotificationCommand {
    Dismiss(u32),
    DismissAll,
    DismissLast,
    InvokeAction(u32, String),
    ToggleDoNotDisturb,
}

#[derive(Clone, Debug, Default)]
struct NotificationState {
    notifications: Vec<NotificationInfo>,
    do_not_disturb: bool,
    next_id: u32,
}

impl NotificationState {
    fn remove_by_id(&mut self, id: u32) -> bool {
        let initial_len = self.notifications.len();
        self.notifications.retain(|n| n.id != id);
        self.notifications.len() != initial_len
    }

    fn remove_last(&mut self) -> bool {
        if self.notifications.is_empty() {
            return false;
        }
        self.notifications.pop();
        true
    }

    fn to_status_message(&self) -> NotificationStatusMessage {
        NotificationStatusMessage::new(self.do_not_disturb, self.notifications.clone(), self.notifications.len() as u32)
    }
}

pub struct NotificationService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: NotificationServiceConfig,
    pub command_sender: Sender<NotificationCommand>,
    pub status_receiver: Arc<Mutex<Receiver<NotificationStatusMessage>>>,
}

impl NotificationService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let notification_config: NotificationServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let (command_sender, command_receiver) = mpsc::channel::<NotificationCommand>();
        let (status_sender, status_receiver) = mpsc::channel::<NotificationStatusMessage>();
        let meta = PluginMeta::try_from(&config)?;
        let config_for_thread = notification_config.clone();
        std::thread::spawn(move || {
            run_notification_thread(command_receiver, status_sender, config_for_thread);
        });
        let service = NotificationService {
            meta,
            core_context,
            config: notification_config,
            command_sender,
            status_receiver: Arc::new(Mutex::new(status_receiver)),
        };
        let meta_clone = service.meta.clone();
        let core_context_clone = service.core_context.clone();
        let status_receiver_clone = Arc::clone(&service.status_receiver);
        glib::timeout_add_local(Duration::from_millis(50), move || {
            while let Ok(status) = status_receiver_clone.lock().unwrap().try_recv() {
                if let Ok(payload) = serde_json::to_string(&status) {
                    let envelope = FfiEnvelope {
                        sender_id: abi_stable::std_types::RString::from(meta_clone.id.clone()),
                        topic: abi_stable::std_types::RString::from(NotificationStatusMessage::topic()),
                        payload: abi_stable::std_types::RString::from(payload),
                    };
                    if let Some(ctx) = &core_context_clone {
                        ctx.send_message(envelope);
                    }
                }
            }
            glib::ControlFlow::Continue
        });
        Ok(service)
    }

    fn handle_dismiss(&self, id: u32) {
        let _ = self.command_sender.send(NotificationCommand::Dismiss(id));
    }

    fn handle_dismiss_all(&self) {
        let _ = self.command_sender.send(NotificationCommand::DismissAll);
    }

    fn handle_dismiss_last(&self) {
        let _ = self.command_sender.send(NotificationCommand::DismissLast);
    }

    fn handle_invoke_action(&self, id: u32, action_key: String) {
        let _ = self.command_sender.send(NotificationCommand::InvokeAction(id, action_key));
    }

    fn handle_toggle_do_not_disturb(&self) {
        let _ = self.command_sender.send(NotificationCommand::ToggleDoNotDisturb);
    }
}

impl MessageHandler<FfiEnvelopePayload<NotificationCommandMessage>> for NotificationService {
    fn handle_message(&self, message: FfiEnvelopePayload<NotificationCommandMessage>, _sender_id: &str) {
        info!("Notification Service: received command {:?}", message.action);
        match message.action {
            NotificationCommandAction::Dismiss => {
                if let Some(id) = message.notification_id {
                    self.handle_dismiss(id);
                }
            }
            NotificationCommandAction::DismissAll => self.handle_dismiss_all(),
            NotificationCommandAction::DismissLast => self.handle_dismiss_last(),
            NotificationCommandAction::InvokeAction => {
                if let Some(id) = message.notification_id {
                    if let Some(ref key) = message.action_key {
                        self.handle_invoke_action(id, key.clone());
                    }
                }
            }
            NotificationCommandAction::ToggleDoNotDisturb => self.handle_toggle_do_not_disturb(),
        }
    }
}

impl MessageBroadcaster<NotificationStatusMessage> for NotificationService {}
impl MessageTopicBroadcaster<NotificationStatusMessage> for NotificationService {}
impl PluginMetaGetter for NotificationService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}
impl AsRef<Option<FfiCoreContext>> for NotificationService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

fn run_notification_thread(
    command_receiver: Receiver<NotificationCommand>,
    status_sender: Sender<NotificationStatusMessage>,
    _config: NotificationServiceConfig,
) {
    debug!("Notification Service: starting notification thread");
    let mut state = NotificationState::default();
    let mut last_broadcast: Option<NotificationStatusMessage> = None;

    let _ = status_sender.send(state.to_status_message());

    loop {
        match command_receiver.recv_timeout(Duration::from_millis(500)) {
            Ok(NotificationCommand::Dismiss(id)) => {
                debug!("Notification Service: dismissing notification {id}");
                state.remove_by_id(id);
            }
            Ok(NotificationCommand::DismissAll) => {
                debug!("Notification Service: dismissing all notifications");
                state.notifications.clear();
            }
            Ok(NotificationCommand::DismissLast) => {
                debug!("Notification Service: dismissing last notification");
                state.remove_last();
            }
            Ok(NotificationCommand::ToggleDoNotDisturb) => {
                state.do_not_disturb = !state.do_not_disturb;
                debug!("Notification Service: Do Not Disturb = {}", state.do_not_disturb);
            }
            Ok(NotificationCommand::InvokeAction(id, key)) => {
                debug!("Notification Service: invoking action {key} on notification {id}");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                debug!("Notification Service: command channel disconnected, exiting thread");
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        let status = state.to_status_message();
        if last_broadcast.as_ref() != Some(&status) {
            let _ = status_sender.send(status.clone());
            last_broadcast = Some(status);
        }
    }
    debug!("Notification Service: notification thread exiting");
}
