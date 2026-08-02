use crate::config::PowerWidgetConfig;
use crate::labels::PowerLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::Button;
use gtk4::Image;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TOPIC_STATUS as TOPIC_PERSONALIZATION_STATUS;
use smearor_power_model::PowerAction;
use smearor_power_model::PowerCommandMessage;
use smearor_power_model::PowerStatusMessage;
use smearor_power_model::TOPIC_STATUS;
use smearor_power_model::power_action_icon;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::GestureHandler;
use smearor_swipe_launcher_plugin_api::GestureHandlersConfiguration;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::apply_icon_color;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::debug;
use tracing::trace;

type SharedLabel = Rc<RefCell<Option<Label>>>;
type SharedButton = Rc<RefCell<Option<Button>>>;
type SharedBox = Rc<RefCell<Option<GtkBox>>>;
type SharedImage = Arc<Mutex<Option<Image>>>;
type SharedLevelBar = Arc<Mutex<Option<LevelBar>>>;

/// Which view the power widget is currently displaying.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetView {
    /// Compact single-action view with icon, label, and optional countdown bar.
    #[default]
    Compact,
    /// Expanded confirmation grid showing all enabled power actions.
    Confirm,
}

pub struct PowerWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: PowerWidgetConfig,
    pub status_sender: tokio::sync::mpsc::UnboundedSender<PowerStatusMessage>,
    pub status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<PowerStatusMessage>>,
    pub inhibitor_label: SharedLabel,
    pub countdown_label: SharedLabel,
    pub scheduled_label: SharedLabel,
    pub action_button: SharedButton,
    pub button_inner: SharedBox,
    pub action_icon: SharedImage,
    pub main_label: SharedLabel,
    pub info_label: SharedLabel,
    pub timeout_bar: SharedLevelBar,
    pub current_view: Rc<RefCell<usize>>,
    pub enabled_actions: Rc<RefCell<Vec<PowerAction>>>,
    pub last_status: Rc<RefCell<Option<PowerStatusMessage>>>,
    pub widget_view: Rc<RefCell<WidgetView>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl PowerWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: PowerWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel::<PowerStatusMessage>();

        let enabled_actions = widget_config.enabled_actions();

        let widget = PowerWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            status_sender,
            status_receiver: Some(status_receiver),
            inhibitor_label: Rc::new(RefCell::new(None)),
            countdown_label: Rc::new(RefCell::new(None)),
            scheduled_label: Rc::new(RefCell::new(None)),
            action_button: Rc::new(RefCell::new(None)),
            button_inner: Rc::new(RefCell::new(None)),
            action_icon: Arc::new(Mutex::new(None)),
            main_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            timeout_bar: Arc::new(Mutex::new(None)),
            current_view: Rc::new(RefCell::new(0)),
            enabled_actions: Rc::new(RefCell::new(enabled_actions)),
            last_status: Rc::new(RefCell::new(None)),
            widget_view: Rc::new(RefCell::new(WidgetView::Compact)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_initial_status();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_initial_status(&self) {
        self.get_broadcaster().broadcast_message_to_topic(PowerCommandMessage::refresh());
    }

    fn request_personalization_status(&self) {
        self.get_broadcaster()
            .broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    /// Broadcast a WidgetUpdateMessage so headless/Web instances re-render this widget.
    pub fn broadcast_widget_update(&self) {
        let plugin_id = self.meta.id.to_string();
        let msg = WidgetUpdateMessage::new(&plugin_id, "");
        self.get_broadcaster().broadcast_message_to_topic(msg);
    }

    /// Returns the list of power actions shown in the Confirm view grid.
    pub fn confirm_actions(&self) -> Vec<PowerAction> {
        self.enabled_actions.borrow().clone()
    }

    /// Switches to the Confirm (expanded grid) view.
    pub fn expand_view(&self) {
        let mut view = self.widget_view.borrow_mut();
        if *view == WidgetView::Confirm {
            return;
        }
        *view = WidgetView::Confirm;
        drop(view);
        self.broadcast_widget_update();
    }

    /// Switches to the Compact (single-action) view.
    pub fn collapse_view(&self) {
        let mut view = self.widget_view.borrow_mut();
        if *view == WidgetView::Compact {
            return;
        }
        *view = WidgetView::Compact;
        drop(view);
        self.broadcast_widget_update();
    }

    /// Toggles between Compact and Confirm views.
    pub fn toggle_view(&self) {
        let current = *self.widget_view.borrow();
        match current {
            WidgetView::Compact => self.expand_view(),
            WidgetView::Confirm => self.collapse_view(),
        }
    }

    fn start_status_listener(&mut self) {
        if let Some(mut receiver) = self.status_receiver.take() {
            let info_label = self.info_label.clone();
            let timeout_bar = self.timeout_bar.clone();
            let show_inhibitors = self.config.show_inhibitor_warnings;
            let current_view = self.current_view.clone();
            let enabled_actions = self.enabled_actions.clone();
            let last_status = self.last_status.clone();
            let main_label = self.main_label.clone();
            let action_icon = self.action_icon.clone();
            let personalization = self.personalization.clone();

            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
                    *last_status.borrow_mut() = Some(status.clone());
                    let action = enabled_actions.borrow().get(*current_view.borrow()).cloned();
                    let override_data = personalization.borrow().clone();
                    update_info_and_timeout(&info_label, &timeout_bar, &status, action.as_ref(), show_inhibitors, &override_data);
                    if let Some(ref act) = action {
                        let icon_name = power_action_icon(act);
                        if let Ok(icon_guard) = action_icon.lock() {
                            if let Some(ref icon) = *icon_guard {
                                set_power_icon(icon, icon_name);
                            }
                        }
                        let label = PowerLabel::from_action(act, override_data.locale);
                        if let Some(ref ml) = *main_label.borrow() {
                            ml.set_text(&label);
                        }
                    }
                }
            });
        }
    }

    fn next_view(&self) {
        self.cycle_view(1);
    }

    fn prev_view(&self) {
        self.cycle_view(-1);
    }

    fn cycle_view(&self, direction: i32) {
        let actions = self.enabled_actions.borrow().clone();
        if actions.len() <= 1 {
            return;
        }
        let mut idx = self.current_view.borrow_mut();
        let len = actions.len() as i32;
        *idx = ((*idx as i32 + direction + len) as usize) % len as usize;
        let action = actions[*idx].clone();
        drop(idx);

        let icon_name = power_action_icon(&action);
        if let Ok(icon_guard) = self.action_icon.lock() {
            if let Some(ref icon) = *icon_guard {
                set_power_icon(icon, icon_name);
            }
        }

        let override_data = self.personalization.borrow().clone();
        let label = PowerLabel::from_action(&action, override_data.locale);
        if let Some(ref ml) = *self.main_label.borrow() {
            ml.set_text(&label);
        }

        if let Some(ref status) = *self.last_status.borrow() {
            update_info_and_timeout(
                &self.info_label,
                &self.timeout_bar,
                status,
                Some(&action),
                self.config.show_inhibitor_warnings,
                &override_data,
            );
        }
        self.broadcast_widget_update();
    }
}

impl DefaultFallback for PowerWidget {
    fn default_fallback(&self, kind: &ActionKind, broadcaster: &MessageBroadcasterInner) {
        match kind {
            ActionKind::Longpress => {
                let actions = self.enabled_actions.borrow();
                let idx = *self.current_view.borrow();
                if let Some(action) = actions.get(idx) {
                    broadcaster.broadcast_message_to_topic(PowerCommandMessage::execute(action.clone()));
                }
            }
            ActionKind::Click | ActionKind::DoublePress => {
                broadcaster.broadcast_message_to_topic(PowerCommandMessage::cancel());
            }
            ActionKind::SwipeUp | ActionKind::ScrollUp | ActionKind::MiddleClick => {
                self.next_view();
            }
            ActionKind::SwipeDown | ActionKind::ScrollDown => {
                self.prev_view();
            }
            ActionKind::RightClick => {
                self.toggle_view();
            }
            ActionKind::Hold | ActionKind::CompoundLongpress | ActionKind::Init => {}
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for PowerWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("power widget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride {
            time_format: Some(status.time_format),
            locale,
        };
        *self.personalization.borrow_mut() = override_data;
        if let Some(ref status) = *self.last_status.borrow() {
            let action = self.enabled_actions.borrow().get(*self.current_view.borrow()).cloned();
            let od = self.personalization.borrow().clone();
            update_info_and_timeout(&self.info_label, &self.timeout_bar, status, action.as_ref(), self.config.show_inhibitor_warnings, &od);
            if let Some(ref act) = action {
                let label = PowerLabel::from_action(act, od.locale);
                if let Some(ref ml) = *self.main_label.borrow() {
                    ml.set_text(&label);
                }
            }
        }
        self.broadcast_widget_update();
    }
}

impl MessageHandler<FfiEnvelopePayload<PowerStatusMessage>> for PowerWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PowerStatusMessage>, _sender_id: &str) {
        if let Err(e) = self.status_sender.send(message.0) {
            debug!("Power Widget: failed to forward status to UI thread: {e}");
        }
    }
}

impl AcceptTopic<FfiEnvelope> for PowerWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == TOPIC_PERSONALIZATION_STATUS || topic == TOPIC_MCP_INVOKE_TOOL
    }
}

impl MessageBroadcaster for PowerWidget {}

impl PluginMetaGetter for PowerWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for PowerWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetPlugin for PowerWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == PowerStatusMessage::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PowerStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<PersonalizationStatusMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == <FfiEnvelopePayload<InvokeToolMessage> as TypedMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for PowerWidget {
    fn build_widget(&mut self) -> Widget {
        let config = self.config.clone();
        let broadcaster = self.get_broadcaster();

        let actions = self.enabled_actions.borrow().clone();
        if let Some(ref default_action) = config.default_action {
            let target = PowerAction::from_str(default_action);
            if let Some(idx) = actions.iter().position(|a| *a == target) {
                *self.current_view.borrow_mut() = idx;
            }
        }
        let current_action = actions.get(*self.current_view.borrow()).cloned().unwrap_or(PowerAction::Shutdown);
        let icon_name = power_action_icon(&current_action);
        let show_labels = !config.icon_config.icon_only();
        let display_name = PowerLabel::from_action(&current_action, Locale::default());

        let button_inner = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(config.layout.spacing_or_default())
            .valign(Align::Center)
            .halign(Align::Center)
            .vexpand(true)
            .css_classes(["menu_button_inner"])
            .build();

        // Line 0: Icon
        let icon = Image::new();
        icon.set_pixel_size(config.icon_config.icon_size());
        icon.add_css_class("nerd-icon");
        set_power_icon(&icon, icon_name);
        if let Some(color) = config.icon_config.icon_color() {
            apply_icon_color(&icon, color);
        }
        button_inner.append(&icon);
        *self.action_icon.lock().unwrap() = Some(icon);

        // Line 1: Main label (action name)
        let main_label = Label::builder()
            .label(if show_labels { &display_name } else { "" })
            .css_classes(["widget-main-text"])
            .build();
        main_label.set_height_request(20);
        apply_text_color(&main_label, config.text_colors.main_text_color());
        button_inner.append(&main_label);
        *self.main_label.borrow_mut() = Some(main_label);

        // Line 2: Info label (countdown/scheduled status)
        let info_label = Label::builder().label("").css_classes(["widget-info-text"]).build();
        info_label.set_height_request(16);
        apply_text_color(&info_label, config.text_colors.info_text_color());
        button_inner.append(&info_label);
        *self.info_label.borrow_mut() = Some(info_label);

        // Line 3: Timeout progress bar or spacer
        match config.mode {
            WidgetMode::Wide => {
                let timeout_bar = LevelBar::builder()
                    .min_value(0.0)
                    .max_value(1.0)
                    .value(0.0)
                    .css_classes(["power-timeout-bar"])
                    .build();
                timeout_bar.set_height_request(16);
                button_inner.append(&timeout_bar);
                *self.timeout_bar.lock().unwrap() = Some(timeout_bar);
            }
            WidgetMode::Compact => {
                let spacer = Label::new(Some(""));
                spacer.set_height_request(16);
                button_inner.append(&spacer);
            }
        }

        *self.button_inner.borrow_mut() = Some(button_inner.clone());

        let effective_width = config.dimensions.width_or_default().min(config.dimensions.max_width_or_default(config.mode));
        let mut button_builder = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(effective_width)
            .child(&button_inner);
        if let Some(max_w) = config.dimensions.max_width {
            button_builder = button_builder.hexpand(false).halign(Align::Start);
            let css_class = format!("max-width-{}", max_w);
            button_builder = button_builder.css_classes(["scroll-item", "menu-button", css_class.as_str()]);
            let css = format!(".max-width-{} {{ max-width: {}px; }}", max_w, max_w);
            if let Some(display) = gtk4::gdk::Display::default() {
                let provider = gtk4::CssProvider::new();
                provider.load_from_string(&css);
                gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
            }
        }
        let button = button_builder.build();

        *self.action_button.borrow_mut() = Some(button.clone());

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            status_sender: self.status_sender.clone(),
            status_receiver: None,
            inhibitor_label: self.inhibitor_label.clone(),
            countdown_label: self.countdown_label.clone(),
            scheduled_label: self.scheduled_label.clone(),
            action_button: self.action_button.clone(),
            button_inner: self.button_inner.clone(),
            action_icon: self.action_icon.clone(),
            main_label: self.main_label.clone(),
            info_label: self.info_label.clone(),
            timeout_bar: self.timeout_bar.clone(),
            current_view: self.current_view.clone(),
            enabled_actions: self.enabled_actions.clone(),
            last_status: self.last_status.clone(),
            widget_view: self.widget_view.clone(),
            personalization: self.personalization.clone(),
        });

        self.start_status_listener();

        let button_widget = button.upcast::<Widget>();
        widget_self.attach_gesture_handlers(
            &button_widget,
            &widget_self.config.actions,
            &broadcaster,
            &GestureHandlersConfiguration {
                longpress_css_class: Some("longpress-active".to_string()),
                group_gestures: false,
                ..Default::default()
            },
        );

        button_widget
    }
}

/// Sets a Nerd Font icon on a `gtk4::Image` by resolving the icon name to a
/// GResource SVG path. Falls back to `set_icon_name` if the resource is not found.
fn set_power_icon(image: &Image, icon_name: &str) {
    if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
        image.set_icon_name(Some(&gtk_icon_name));
        return;
    }
    image.set_icon_name(Some(icon_name));
}

/// Updates the info label and timeout progress bar based on inhibitor warnings,
/// countdown, or scheduled action status.
/// Priority: inhibitor warning > countdown > scheduled > empty.
fn update_info_and_timeout(
    info_label: &SharedLabel,
    timeout_bar: &SharedLevelBar,
    status: &PowerStatusMessage,
    current_action: Option<&PowerAction>,
    show_inhibitors: bool,
    override_data: &PersonalizationOverride,
) {
    let info_text = if show_inhibitors {
        let what_filter = current_action.map(|a| a.inhibitor_what()).unwrap_or("");
        let relevant: Vec<String> = status
            .inhibitors
            .iter()
            .filter(|inh| what_filter.is_empty() || inh.what.to_lowercase().contains(what_filter))
            .map(|inh| format!("{}: {}", inh.who.to_string(), inh.reason.to_string()))
            .collect();
        if relevant.is_empty() {
            None
        } else {
            let text = format!("\u{f0027} {}", relevant.join(", "));
            Some(if text.chars().count() > 40 {
                let truncated: String = text.chars().take(37).collect();
                format!("{truncated}...")
            } else {
                text
            })
        }
    } else {
        None
    };

    let (info_text, fraction, bar_visible) = if let Some(inh_text) = info_text {
        (inh_text, 0.0, false)
    } else if status.countdown_active {
        let remaining = status.countdown_remaining_seconds as f64;
        let total = status.countdown_total_seconds as f64;
        let frac = if total > 0.0 { remaining / total } else { 0.0 };
        let label = PowerLabel::ShuttingDown.localized_label(override_data.locale);
        (PowerLabel::format_with_seconds(&label, status.countdown_remaining_seconds), frac, true)
    } else if let Some(sched) = status.scheduled_action.as_ref() {
        let remaining = sched.remaining_seconds as f64;
        let total = sched.total_delay_seconds as f64;
        let frac = if total > 0.0 { remaining / total } else { 0.0 };
        (override_data.format_countdown(sched.remaining_seconds), frac, true)
    } else {
        (String::new(), 0.0, false)
    };

    if let Ok(bar_guard) = timeout_bar.lock() {
        if let Some(ref bar) = *bar_guard {
            bar.set_value(fraction);
            bar.set_opacity(if bar_visible { 1.0 } else { 0.0 });
        }
    }
    if let Some(ref label) = *info_label.borrow() {
        label.set_text(&info_text);
    }
}
