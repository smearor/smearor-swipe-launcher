use crate::config::NotificationServiceConfig;
use futures_util::StreamExt;
use smearor_notifications_model::NotificationAction;
use smearor_notifications_model::NotificationCommandAction;
use smearor_notifications_model::NotificationCommandMessage;
use smearor_notifications_model::NotificationInfo;
use smearor_notifications_model::NotificationStatusMessage;
use smearor_notifications_model::UrgencyLevel;
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
use smearor_swipe_launcher_plugin_api::Service;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use tracing::debug;
use tracing::error;
use tracing::info;
use zbus::Connection;
use zbus::MessageStream;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::Value;

#[derive(Debug)]
pub enum NotificationCommand {
    Dismiss(u32),
    DismissAll,
    DismissLast,
    InvokeAction(u32, String),
    ToggleDoNotDisturb,
}

#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait ExternalNotifications {
    fn close_notification(&self, id: u32) -> zbus::Result<()>;
}

#[derive(Debug)]
enum SignalCommand {
    NotificationClosed(u32, u32),
    ActionInvoked(u32, String),
}

#[derive(Clone, Debug, Default)]
struct NotificationState {
    notifications: BTreeMap<u32, NotificationInfo>,
    do_not_disturb: bool,
    next_id: u32,
}

impl NotificationState {
    fn remove_by_id(&mut self, id: u32) -> bool {
        self.notifications.remove(&id).is_some()
    }

    fn remove_last(&mut self) -> bool {
        if self.notifications.is_empty() {
            return false;
        }
        let last_id = self.notifications.last_key_value().map(|(k, _)| *k);
        if let Some(id) = last_id {
            self.notifications.remove(&id);
            true
        } else {
            false
        }
    }

    fn to_status_message(&self) -> NotificationStatusMessage {
        let list: stabby::vec::Vec<NotificationInfo> = self.notifications.values().cloned().collect();
        NotificationStatusMessage::new(self.do_not_disturb, list, self.notifications.len() as u32)
    }
}

struct Notifications {
    state: Arc<Mutex<NotificationState>>,
    status_sender: tokio::sync::mpsc::Sender<NotificationStatusMessage>,
    signal_sender: tokio::sync::mpsc::Sender<SignalCommand>,
}

#[interface(name = "org.freedesktop.Notifications")]
impl Notifications {
    async fn notify(
        &mut self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, Value<'_>>,
        expire_timeout: i32,
    ) -> u32 {
        let (id, status) = {
            let mut state = self.state.lock().unwrap();

            let id = if replaces_id > 0 {
                replaces_id
            } else {
                state.next_id += 1;
                state.next_id
            };

            let urgency = match hints.get("urgency") {
                Some(Value::U8(u)) => match *u {
                    0 => UrgencyLevel::Low,
                    2 => UrgencyLevel::Critical,
                    _ => UrgencyLevel::Normal,
                },
                _ => UrgencyLevel::Normal,
            };

            let mut parsed_actions = stabby::vec::Vec::new();
            let mut action_iter = actions.chunks_exact(2);
            while let Some(chunk) = action_iter.next() {
                parsed_actions.push(NotificationAction {
                    key: stabby::string::String::from(chunk[0].clone()),
                    label: stabby::string::String::from(chunk[1].clone()),
                });
            }

            let notification = NotificationInfo {
                id,
                app_name: stabby::string::String::from(app_name),
                summary: stabby::string::String::from(summary),
                body: stabby::string::String::from(body),
                icon: if app_icon.is_empty() {
                    stabby::option::Option::None()
                } else {
                    stabby::option::Option::Some(stabby::string::String::from(app_icon))
                },
                urgency,
                actions: parsed_actions,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                timeout_ms: expire_timeout,
            };

            state.notifications.insert(id, notification);

            let status = state.to_status_message();
            (id, status)
        };

        let _ = self.status_sender.send(status).await;

        id
    }

    async fn close_notification(&mut self, id: u32) -> zbus::fdo::Result<()> {
        let status = {
            let mut state = self.state.lock().unwrap();
            state.remove_by_id(id);
            state.to_status_message()
        };
        let _ = self.status_sender.send(status).await;
        let _ = self.signal_sender.send(SignalCommand::NotificationClosed(id, 3)).await;
        Ok(())
    }

    #[zbus(out_args("name", "vendor", "version", "spec_version"))]
    async fn get_server_information(&self) -> (String, String, String, String) {
        ("smearor-notifications".to_string(), "smearor".to_string(), "0.1.0".to_string(), "1.2".to_string())
    }

    async fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".to_string(),
            "body-hyperlinks".to_string(),
            "actions".to_string(),
            "icon-static".to_string(),
            "persistence".to_string(),
        ]
    }

    #[zbus(signal)]
    async fn notification_closed(&self, _ctxt: &SignalEmitter<'_>, id: u32, reason: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn action_invoked(&self, _ctxt: &SignalEmitter<'_>, id: u32, action_key: &str) -> zbus::Result<()>;
}

pub struct NotificationService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: NotificationServiceConfig,
    pub command_sender: tokio::sync::mpsc::Sender<NotificationCommand>,
    pub command_receiver: Option<tokio::sync::mpsc::Receiver<NotificationCommand>>,
}

impl NotificationService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_notifications_model::register_json_converters(core_context);

        let notification_config: NotificationServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let (command_sender, command_receiver) = tokio::sync::mpsc::channel::<NotificationCommand>(100);
        let meta = PluginMeta::try_from(&config)?;
        let service = NotificationService {
            meta,
            core_context,
            config: notification_config,
            command_sender,
            command_receiver: Some(command_receiver),
        };
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
                if let Some(id) = message.notification_id.as_ref().copied() {
                    self.handle_dismiss(id);
                }
            }
            NotificationCommandAction::DismissAll => self.handle_dismiss_all(),
            NotificationCommandAction::DismissLast => self.handle_dismiss_last(),
            NotificationCommandAction::InvokeAction => {
                if let Some(id) = message.notification_id.as_ref().copied() {
                    if let Some(key) = message.action_key.as_ref() {
                        self.handle_invoke_action(id, key.as_str().to_string());
                    }
                }
            }
            NotificationCommandAction::ToggleDoNotDisturb => self.handle_toggle_do_not_disturb(),
        }
    }
}

impl MessageBroadcaster for NotificationService {}
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

impl Service for NotificationService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == FfiEnvelopePayload::<NotificationCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<NotificationCommandMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }

    fn start(&mut self) {
        if let Some(ctx) = &self.core_context {
            let meta = self.meta.clone();
            let core_context = *ctx;
            let command_receiver = self.command_receiver.take();
            let config = self.config.clone();
            std::thread::spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => rt,
                    Err(e) => {
                        error!("Notification Service: failed to create tokio runtime: {e}");
                        return;
                    }
                };
                rt.block_on(async move {
                    if let Some(receiver) = command_receiver {
                        run_notification_async(meta, core_context, receiver, config).await;
                    }
                });
            });
        }
    }
}

fn send_status(meta: &PluginMeta, core_context: &FfiCoreContext, status: NotificationStatusMessage) {
    let payload_ptr = Box::into_raw(Box::new(status)) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: stabby::string::String::from(meta.id.clone()),
        target_instance_id: stabby::string::String::from("*"),
        topic: stabby::string::String::from(NotificationStatusMessage::topic()),
        type_id: NotificationStatusMessage::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(destroy_notification_status),
        clone_payload: Some(clone_notification_status),
    };
    core_context.send_message(envelope);
}

async fn run_notification_async(
    meta: PluginMeta,
    core_context: FfiCoreContext,
    mut command_receiver: tokio::sync::mpsc::Receiver<NotificationCommand>,
    _config: NotificationServiceConfig,
) {
    debug!("Notification Service: starting notification async task");
    let state = Arc::new(Mutex::new(NotificationState::default()));

    let (signal_sender, mut signal_receiver) = tokio::sync::mpsc::channel::<SignalCommand>(100);
    let (status_sender, mut status_receiver) = tokio::sync::mpsc::channel::<NotificationStatusMessage>(100);

    let forward_meta = meta.clone();
    let forward_core = core_context.clone();
    tokio::task::spawn_local(async move {
        while let Some(status) = status_receiver.recv().await {
            send_status(&forward_meta, &forward_core, status);
        }
    });

    let notifications = Notifications {
        state: Arc::clone(&state),
        status_sender: status_sender.clone(),
        signal_sender: signal_sender.clone(),
    };
    let conn = match Connection::session().await {
        Ok(c) => c,
        Err(e) => {
            error!("Notification Service: failed to create D-Bus connection: {e}");
            return;
        }
    };

    if let Err(e) = conn.object_server().at("/org/freedesktop/Notifications", notifications).await {
        error!("Notification Service: failed to serve at path: {e}");
        return;
    }

    let is_primary_daemon = match conn.request_name("org.freedesktop.Notifications").await {
        Ok(_) => {
            debug!("Notification Service: registered as org.freedesktop.Notifications");
            true
        }
        Err(e) => {
            error!("Notification Service: failed to request bus name (another daemon may be running): {e}");
            false
        }
    };
    let proxy_sender = 'proxy_block: {
        if is_primary_daemon {
            let conn_for_signals = conn.clone();
            tokio::task::spawn_local(async move {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let iface_ref = match conn_for_signals
                    .object_server()
                    .interface::<_, Notifications>("/org/freedesktop/Notifications")
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        error!("Notification Service: failed to get interface ref for signals: {e}");
                        return;
                    }
                };
                while let Some(cmd) = signal_receiver.recv().await {
                    match cmd {
                        SignalCommand::NotificationClosed(id, reason) => {
                            let iface = iface_ref.get().await;
                            if let Err(e) = iface.notification_closed(&iface_ref.signal_emitter(), id, reason).await {
                                error!("Notification Service: failed to emit NotificationClosed: {e}");
                            } else {
                                debug!("Notification Service: emitted NotificationClosed {id} reason={reason}");
                            }
                        }
                        SignalCommand::ActionInvoked(id, action_key) => {
                            let iface = iface_ref.get().await;
                            if let Err(e) = iface.action_invoked(&iface_ref.signal_emitter(), id, &action_key).await {
                                error!("Notification Service: failed to emit ActionInvoked: {e}");
                            } else {
                                debug!("Notification Service: emitted ActionInvoked {id} key={action_key}");
                            }
                        }
                    }
                }
            });
            break 'proxy_block None;
        }

        tokio::task::spawn_local(async move { while signal_receiver.recv().await.is_some() {} });
        debug!("Notification Service: running in monitor mode");

        let (proxy_sender, mut proxy_receiver) = tokio::sync::mpsc::channel::<u32>(100);
        let proxy = match ExternalNotificationsProxy::new(&conn).await {
            Ok(p) => p,
            Err(e) => {
                error!("Notification Service: failed to create proxy to existing daemon: {e}");
                break 'proxy_block None;
            }
        };
        tokio::task::spawn_local(async move {
            while let Some(id) = proxy_receiver.recv().await {
                if let Err(e) = proxy.close_notification(id).await {
                    error!("Notification Service: failed to forward CloseNotification to existing daemon: {e}");
                } else {
                    debug!("Notification Service: forwarded CloseNotification {id} to existing daemon");
                }
            }
        });

        let monitor_conn = match Connection::session().await {
            Ok(c) => c,
            Err(e) => {
                error!("Notification Service: failed to create monitoring D-Bus connection: {e}");
                break 'proxy_block Some(proxy_sender);
            }
        };
        let monitoring = match zbus::fdo::MonitoringProxy::new(&monitor_conn).await {
            Ok(p) => p,
            Err(e) => {
                error!("Notification Service: failed to create Monitoring proxy: {e}");
                break 'proxy_block Some(proxy_sender);
            }
        };

        if let Err(e) = monitoring.become_monitor(&[], 0).await {
            error!("Notification Service: failed to become monitor: {e}");
        } else {
            debug!("Notification Service: D-Bus monitor active on separate connection");

            let monitor_state = Arc::clone(&state);
            let monitor_meta = meta.clone();
            let monitor_core = core_context.clone();
            let conn_for_monitor = monitor_conn.clone();
            let pending_notifications: Arc<Mutex<HashMap<u32, NotificationInfo>>> = Arc::new(Mutex::new(HashMap::new()));
            tokio::task::spawn_local(async move {
                let mut stream = Box::pin(MessageStream::from(conn_for_monitor));
                while let Some(msg) = stream.next().await {
                    let msg = match msg {
                        Ok(m) => m,
                        Err(e) => {
                            debug!("Notification Service: monitor stream error: {e}");
                            continue;
                        }
                    };
                    let header = msg.header();
                    match header.message_type() {
                        zbus::message::Type::MethodCall => {
                            let Some(iface) = header.interface() else { continue };
                            if iface != "org.freedesktop.Notifications" {
                                continue;
                            }
                            let Some(member) = header.member() else { continue };
                            if member != "Notify" {
                                continue;
                            }
                            let body = msg.body();
                            let (app_name, replaces_id, app_icon, summary, body_text, actions, _hints, expire_timeout): (
                                String,
                                u32,
                                String,
                                String,
                                String,
                                Vec<String>,
                                HashMap<String, Value>,
                                i32,
                            ) = match body.deserialize() {
                                Ok(v) => v,
                                Err(e) => {
                                    debug!("Notification Service: failed to deserialize Notify body: {e}");
                                    continue;
                                }
                            };

                            let mut parsed_actions = stabby::vec::Vec::new();
                            let mut action_iter = actions.chunks_exact(2);
                            while let Some(chunk) = action_iter.next() {
                                parsed_actions.push(NotificationAction {
                                    key: stabby::string::String::from(chunk[0].clone()),
                                    label: stabby::string::String::from(chunk[1].clone()),
                                });
                            }

                            let serial: u32 = header.primary().serial_num().get();
                            let notification = NotificationInfo {
                                id: 0,
                                app_name: stabby::string::String::from(app_name.clone()),
                                summary: stabby::string::String::from(summary.clone()),
                                body: stabby::string::String::from(body_text.clone()),
                                icon: if app_icon.is_empty() {
                                    stabby::option::Option::None()
                                } else {
                                    stabby::option::Option::Some(stabby::string::String::from(app_icon))
                                },
                                urgency: UrgencyLevel::Normal,
                                actions: parsed_actions,
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64,
                                timeout_ms: expire_timeout,
                            };

                            if replaces_id > 0 {
                                let mut guard = monitor_state.lock().unwrap();
                                let mut n = notification;
                                n.id = replaces_id;
                                guard.notifications.insert(replaces_id, n);
                                let status = guard.to_status_message();
                                let count = guard.notifications.len();
                                drop(guard);
                                send_status(&monitor_meta, &monitor_core, status);
                                debug!("Notification Service: monitor replaced notification id={replaces_id} app={app_name} total={count}");
                            } else {
                                let mut pending = pending_notifications.lock().unwrap();
                                pending.insert(serial, notification);
                                debug!("Notification Service: monitor pending notification serial={serial} app={app_name}");
                            }
                        }
                        zbus::message::Type::MethodReturn => {
                            let reply_serial = match header.reply_serial() {
                                Some(s) => s.get(),
                                None => continue,
                            };
                            let mut pending = pending_notifications.lock().unwrap();
                            let Some(notification) = pending.remove(&reply_serial) else {
                                continue;
                            };
                            drop(pending);
                            let id: u32 = match msg.body().deserialize() {
                                Ok(v) => v,
                                Err(e) => {
                                    debug!("Notification Service: failed to deserialize Notify return id: {e}");
                                    continue;
                                }
                            };
                            let mut guard = monitor_state.lock().unwrap();
                            let mut n = notification;
                            n.id = id;
                            guard.notifications.insert(id, n);
                            let status = guard.to_status_message();
                            let count = guard.notifications.len();
                            drop(guard);
                            send_status(&monitor_meta, &monitor_core, status);
                            debug!("Notification Service: monitor added notification id={id} total={count}");
                        }
                        zbus::message::Type::Signal => {
                            let Some(iface) = header.interface() else { continue };
                            if iface != "org.freedesktop.Notifications" {
                                continue;
                            }
                            let Some(member) = header.member() else { continue };
                            if member != "NotificationClosed" {
                                continue;
                            }
                            if let Ok((id, _reason)) = msg.body().deserialize::<(u32, u32)>() {
                                let mut guard = monitor_state.lock().unwrap();
                                guard.remove_by_id(id);
                                let status = guard.to_status_message();
                                drop(guard);
                                send_status(&monitor_meta, &monitor_core, status);
                                debug!("Notification Service: monitor removed notification id={id}");
                            }
                        }
                        _ => {}
                    }
                }
                debug!("Notification Service: monitor stream ended");
            });
        }
        Some(proxy_sender)
    };

    let initial_status = state.lock().unwrap().to_status_message();
    send_status(&meta, &core_context, initial_status.clone());
    let mut last_broadcast = Some(initial_status);

    let mut last_periodic_broadcast = Instant::now();
    const PERIODIC_INTERVAL: Duration = Duration::from_secs(5);

    loop {
        if Instant::now().duration_since(last_periodic_broadcast) >= PERIODIC_INTERVAL {
            let status = state.lock().unwrap().to_status_message();
            send_status(&meta, &core_context, status);
            last_periodic_broadcast = Instant::now();
        }

        match tokio::time::timeout(Duration::from_millis(500), command_receiver.recv()).await {
            Ok(Some(NotificationCommand::Dismiss(id))) => {
                debug!("Notification Service: dismissing notification {id}");
                state.lock().unwrap().remove_by_id(id);
                if let Some(ref sender) = proxy_sender {
                    if let Err(e) = sender.try_send(id) {
                        error!("Notification Service: failed to forward dismiss to proxy: {e}");
                    }
                } else if let Err(e) = signal_sender.send(SignalCommand::NotificationClosed(id, 2)).await {
                    error!("Notification Service: failed to queue NotificationClosed signal: {e}");
                }
            }
            Ok(Some(NotificationCommand::DismissAll)) => {
                debug!("Notification Service: dismissing all notifications");
                let ids: Vec<u32> = state.lock().unwrap().notifications.keys().copied().collect();
                state.lock().unwrap().notifications.clear();
                if let Some(ref sender) = proxy_sender {
                    for id in ids {
                        if let Err(e) = sender.try_send(id) {
                            error!("Notification Service: failed to forward dismiss to proxy: {e}");
                        }
                    }
                } else {
                    for id in ids {
                        if let Err(e) = signal_sender.send(SignalCommand::NotificationClosed(id, 2)).await {
                            error!("Notification Service: failed to queue NotificationClosed signal: {e}");
                        }
                    }
                }
            }
            Ok(Some(NotificationCommand::DismissLast)) => {
                debug!("Notification Service: dismissing last notification");
                let removed_id = state.lock().unwrap().notifications.last_key_value().map(|(k, _)| *k);
                state.lock().unwrap().remove_last();
                if let Some(id) = removed_id {
                    if let Some(ref sender) = proxy_sender {
                        if let Err(e) = sender.try_send(id) {
                            error!("Notification Service: failed to forward dismiss to proxy: {e}");
                        }
                    } else if let Err(e) = signal_sender.send(SignalCommand::NotificationClosed(id, 2)).await {
                        error!("Notification Service: failed to queue NotificationClosed signal: {e}");
                    }
                }
            }
            Ok(Some(NotificationCommand::ToggleDoNotDisturb)) => {
                let mut s = state.lock().unwrap();
                s.do_not_disturb = !s.do_not_disturb;
                debug!("Notification Service: Do Not Disturb = {}", s.do_not_disturb);
            }
            Ok(Some(NotificationCommand::InvokeAction(id, key))) => {
                debug!("Notification Service: invoking action {key} on notification {id}");
                if let Err(e) = signal_sender.send(SignalCommand::ActionInvoked(id, key)).await {
                    error!("Notification Service: failed to queue ActionInvoked signal: {e}");
                }
            }
            Ok(None) => {
                debug!("Notification Service: command channel disconnected, exiting task");
                break;
            }
            Err(_) => {}
        }

        let status = state.lock().unwrap().to_status_message();
        if last_broadcast.as_ref() != Some(&status) {
            send_status(&meta, &core_context, status.clone());
            last_broadcast = Some(status);
        }
    }

    let _ = conn;
    debug!("Notification Service: notification async task exiting");
}

extern "C" fn clone_notification_status(ptr: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let status = unsafe { &*(ptr as *const NotificationStatusMessage) };
    Box::into_raw(Box::new(status.clone())) as *mut core::ffi::c_void
}

extern "C" fn destroy_notification_status(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut NotificationStatusMessage);
        }
    }
}
