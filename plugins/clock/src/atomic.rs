use crate::clock::Clock;
use crate::clock::PersonalizationOverride;
use crate::config::ClockConfig;
use crate::countdown_state::CountdownState;
use crate::countdown_state::format_duration;
use crate::span_state::SpanGroupState;
use crate::span_state::cleanup_state;
use crate::span_state::lookup_or_create_state;
use crate::timer_state::TimerState;
use crate::timer_state::format_elapsed;
use gtk4::Label;
use gtk4::glib::MainContext;
use smearor_model_widget::AtomicWidgetConfig;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_progress_bar;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::AtomicAction;
use smearor_swipe_launcher_plugin_api::AtomicGraphicData;
use smearor_swipe_launcher_plugin_api::AtomicGraphicRenderer;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::SpanActionHandler;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::atomic_widget_impl;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tracing::trace;

/// Which clock view an atomic widget renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockAtomicView {
    /// Digital time display (HH:MM).
    TimeDigital,
    /// Digital date display (DD.MM).
    DateDigital,
    /// Analog clock face.
    TimeAnalog,
    /// Multi-span big digital time display (HH:MM) across multiple buttons.
    BigDigital,
    /// Multi-span big date display across multiple buttons.
    BigDate,
    /// Multi-span countdown timer (MM:SS) across multiple buttons.
    Countdown,
    /// Multi-span stopwatch timer (MM:SS) across multiple buttons.
    Timer,
}

impl FromStr for ClockAtomicView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "clock_time_digital" => Ok(Self::TimeDigital),
            "clock_date_digital" => Ok(Self::DateDigital),
            "clock_time_analog" => Ok(Self::TimeAnalog),
            "clock_big_digital" => Ok(Self::BigDigital),
            "clock_big_date" => Ok(Self::BigDate),
            "clock_countdown" => Ok(Self::Countdown),
            "clock_timer" => Ok(Self::Timer),
            _ => Err(format!("Unknown clock atomic view: {s}")),
        }
    }
}

impl ClockAtomicView {
    /// Returns the default nerd font icon name for this view.
    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::TimeDigital => "nf-fa-clock_o",
            Self::DateDigital => "nf-md-calendar",
            Self::TimeAnalog => "nf-fa-clock_o",
            Self::BigDigital => "nf-fa-clock_o",
            Self::BigDate => "nf-md-calendar",
            Self::Countdown => "nf-md-timer_outline",
            Self::Timer => "nf-md-timer",
        }
    }

    /// Renders this view's display data from the shared clock and personalization override.
    pub fn render(&self, clock: &Clock, override_data: &PersonalizationOverride) -> ViewData {
        clock.update_personalization(override_data.clone());
        match self {
            Self::TimeDigital => {
                let time = clock.get_time_string();
                ViewData::new(self.icon_name().to_string(), time, "".to_string())
            }
            Self::DateDigital => {
                let date = clock.get_date_string();
                let weekday = clock.get_weekday_name();
                ViewData::new(self.icon_name().to_string(), date, weekday.to_string())
            }
            Self::TimeAnalog => {
                let time = clock.get_time_string();
                ViewData::new(self.icon_name().to_string(), time, "".to_string())
            }
            Self::BigDigital => {
                let time = clock.get_time_string();
                ViewData::new(self.icon_name().to_string(), time, "".to_string())
            }
            Self::BigDate => {
                let date = clock.get_date_string();
                ViewData::new(self.icon_name().to_string(), date, "".to_string())
            }
            Self::Countdown => ViewData::new(self.icon_name().to_string(), "00:00".to_string(), "".to_string()),
            Self::Timer => ViewData::new(self.icon_name().to_string(), "00:00".to_string(), "".to_string()),
        }
    }
}

/// Atomic clock widget that renders a single clock view.
///
/// Unlike service-backed atomic widgets (weather, audio), the clock has no
/// service to subscribe to. Time data is generated locally via the shared
/// `Clock` struct. The widget subscribes to personalization status for
/// timezone/locale/format overrides.
pub struct ClockAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AtomicWidgetConfig,
    pub view: ClockAtomicView,
    pub clock: Arc<Clock>,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub main_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
    pub span_group: Option<String>,
    pub span_index: u32,
    pub span_rows: u32,
    pub span_cols: u32,
    pub shared_state: Arc<Mutex<SpanGroupState>>,
}

impl ClockAtomicWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: AtomicWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget_name = config.config.get("widget").and_then(|v| v.as_str()).unwrap_or_default();

        let view = ClockAtomicView::from_str(widget_name).unwrap_or(ClockAtomicView::TimeDigital);

        let clock_config = ClockConfig::default();
        let (time_sender, time_receiver) = tokio::sync::mpsc::unbounded_channel::<()>();

        let span_group = config.config.get("span_group").and_then(|v| v.as_str()).map(|s| s.to_string());
        let span_index = config.config.get("span_index").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(0);
        let span_rows = config.config.get("span_rows").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(1);
        let span_cols = config.config.get("span_cols").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(1);

        let initial_state = match view {
            ClockAtomicView::Timer => SpanGroupState::Timer(TimerState::default()),
            ClockAtomicView::Countdown => SpanGroupState::Countdown(CountdownState::default()),
            _ => SpanGroupState::default(),
        };
        let shared_state = lookup_or_create_state(span_group.as_deref(), initial_state);

        let widget = ClockAtomicWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            view,
            clock: Arc::new(Clock::new(clock_config)),
            icon_label: Rc::new(RefCell::new(None)),
            main_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
            span_group: span_group.clone(),
            span_index,
            span_rows,
            span_cols,
            shared_state: shared_state.clone(),
        };
        widget.register_mcp_capabilities();
        widget.request_personalization_status();
        widget.start_time_update(time_sender, time_receiver);
        Ok(widget)
    }

    fn start_time_update(&self, time_sender: tokio::sync::mpsc::UnboundedSender<()>, mut time_receiver: tokio::sync::mpsc::UnboundedReceiver<()>) {
        thread::spawn(move || {
            loop {
                if time_sender.send(()).is_err() {
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }
        });

        let icon_label = self.icon_label.clone();
        let main_label = self.main_label.clone();
        let info_label = self.info_label.clone();
        let clock = self.clock.clone();
        let personalization = self.personalization.clone();
        let view = self.view;
        let span_index = self.span_index;
        let shared_state = self.shared_state.clone();
        let broadcaster = MessageBroadcasterInner {
            meta: self.meta.clone(),
            core_context: self.core_context,
        };

        let context = MainContext::default();
        if context.is_owner() {
            context.spawn_local(async move {
                while time_receiver.recv().await.is_some() {
                    let mut display_text = String::new();
                    let mut should_broadcast = true;

                    match view {
                        ClockAtomicView::Timer => {
                            if span_index == 0 {
                                let state = shared_state.lock().unwrap();
                                let elapsed = if let Some(timer) = state.as_timer_ref() {
                                    timer.current_elapsed()
                                } else {
                                    Duration::ZERO
                                };
                                display_text = format_elapsed(elapsed);
                            } else {
                                should_broadcast = false;
                            }
                        }
                        ClockAtomicView::Countdown => {
                            if span_index == 0 {
                                let mut state = shared_state.lock().unwrap();
                                if let Some(countdown) = state.as_countdown() {
                                    countdown.tick();
                                    display_text = format_duration(countdown.current_remaining());
                                } else {
                                    display_text = "00:00".to_string();
                                }
                            } else {
                                should_broadcast = false;
                            }
                        }
                        _ => {
                            let override_data = personalization.borrow().clone();
                            let view_data = view.render(&clock, &override_data);
                            display_text = view_data.main_text;
                        }
                    }

                    if should_broadcast {
                        let icon_name = view.icon_name();
                        let icon_char = resolve_icon_codepoint(icon_name).unwrap_or('\u{f017}');
                        smearor_swipe_launcher_plugin_api::update_labels(
                            &*icon_label.borrow(),
                            &*main_label.borrow(),
                            &*info_label.borrow(),
                            &icon_char.to_string(),
                            &display_text,
                            "",
                        );
                        broadcaster.broadcast_message_to_topic(WidgetUpdateMessage::new(&broadcaster.meta.id.to_string(), ""));
                    }
                }
            });
        }
    }

    fn request_personalization_status(&self) {
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    pub fn update_ui(&self) {
        let view_data = self.render_view_data();
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f017}');
        smearor_swipe_launcher_plugin_api::update_labels(
            &*self.icon_label.borrow(),
            &*self.main_label.borrow(),
            &*self.info_label.borrow(),
            &icon_char.to_string(),
            &view_data.main_text,
            &view_data.info_text,
        );
        if let Some(ref label) = *self.main_label.borrow() {
            apply_text_color(label, self.config.text_colors.main_text_color());
        }
        if let Some(ref label) = *self.info_label.borrow() {
            apply_text_color(label, self.config.text_colors.info_text_color());
        }
    }

    /// Render the current view data from the shared clock or shared span state.
    fn render_view_data(&self) -> ViewData {
        match self.view {
            ClockAtomicView::Timer => {
                let state = self.shared_state.lock().unwrap();
                let elapsed = if let Some(timer) = state.as_timer_ref() {
                    timer.current_elapsed()
                } else {
                    Duration::ZERO
                };
                let display = format_elapsed(elapsed);
                ViewData::new(self.view.icon_name().to_string(), display, "".to_string())
            }
            ClockAtomicView::Countdown => {
                let state = self.shared_state.lock().unwrap();
                let remaining = if let Some(countdown) = state.as_countdown_ref() {
                    countdown.current_remaining()
                } else {
                    Duration::ZERO
                };
                let display = format_duration(remaining);
                ViewData::new(self.view.icon_name().to_string(), display, "".to_string())
            }
            _ => {
                let override_data = self.personalization.borrow().clone();
                self.view.render(&self.clock, &override_data)
            }
        }
    }

    /// Extract graphic rendering data for the centralised rendering pipeline.
    fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
        let view_data = self.render_view_data();
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f017}');
        let mut data = AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text);
        data.main_text_color = self.config.text_colors.main_text_color().map(|c| c.to_rgba());
        data.info_text_color = self.config.text_colors.info_text_color().map(|c| c.to_rgba());
        data
    }
}

atomic_widget_impl! {
    widget: ClockAtomicWidget,
    debug_tag: "clock-atomic",
    mcp_description: "Clock atomic widget",
    css_prefix: "clock",
    default_icon: '\u{f017}',
    default_main: "--:--",
    default_info: "Loading...",
    graphic_renderer: true,
    extra_message_types: [FfiEnvelopePayload<PersonalizationStatusMessage>],
    span_action_handler: true,
}

impl AtomicGraphicRenderer for ClockAtomicWidget {
    fn render_graphic(&self, pixels: &mut [u8], width: u32, height: u32) -> bool {
        let is_multi_span = matches!(
            self.view,
            ClockAtomicView::BigDigital | ClockAtomicView::BigDate | ClockAtomicView::Countdown | ClockAtomicView::Timer
        );
        if !is_multi_span {
            return false;
        }

        let text_col = text_color(false);
        fill_background(pixels, width, height, background_color(false));

        let view_data = self.render_view_data();

        if self.view == ClockAtomicView::BigDate && self.span_rows >= 2 {
            let (day_month, year) = self.clock.get_date_parts();
            let line1_font = (height as f32 * 0.35).min(width as f32 * 0.25).max(16.0);
            let line2_font = (height as f32 * 0.25).min(width as f32 * 0.2).max(14.0);
            draw_text_centered(pixels, width, height, &day_month, height as f32 * 0.35, line1_font, text_col);
            draw_text_centered(pixels, width, height, &year, height as f32 * 0.72, line2_font, text_col);
        } else {
            let char_count = view_data.main_text.len().max(1) as f32;
            let max_font_by_width = width as f32 / char_count * 1.6;
            let font_size = (height as f32 * 0.55).min(max_font_by_width).max(16.0);
            draw_text_centered(pixels, width, height, &view_data.main_text, height as f32 * 0.55, font_size, text_col);
        }

        if self.view == ClockAtomicView::Countdown {
            let state = self.shared_state.lock().unwrap();
            if let Some(countdown) = state.as_countdown_ref() {
                if countdown.target > std::time::Duration::ZERO {
                    let remaining = countdown.current_remaining();
                    let progress = remaining.as_secs_f32() / countdown.target.as_secs_f32();
                    draw_progress_bar(pixels, width, height, progress, text_col);
                }
            }
        }

        true
    }
}

impl SpanActionHandler for ClockAtomicWidget {
    fn on_span_action(&self, action: AtomicAction, span_index: u32) {
        match self.view {
            ClockAtomicView::Timer => {
                let mut state = self.shared_state.lock().unwrap();
                if let Some(timer) = state.as_timer() {
                    match (action, span_index) {
                        (AtomicAction::Click, 0) => timer.start(),
                        (AtomicAction::Click, 1) => timer.pause(),
                        (AtomicAction::Click, 2) => timer.reset(),
                        (AtomicAction::CompoundLongpress, _) => timer.reset(),
                        _ => {}
                    }
                }
                drop(state);
                self.update_ui();
                self.broadcast_widget_update();
            }
            ClockAtomicView::Countdown => {
                let mut state = self.shared_state.lock().unwrap();
                if let Some(countdown) = state.as_countdown() {
                    match (action, span_index) {
                        (AtomicAction::Click, 0) => countdown.increment_minutes(1),
                        (AtomicAction::Click, 1) => countdown.increment_seconds(1),
                        (AtomicAction::Click, 2) => countdown.reset(),
                        (AtomicAction::Longpress, 0) => countdown.start(),
                        (AtomicAction::Longpress, 1) => countdown.toggle_pause(),
                        (AtomicAction::CompoundLongpress, _) => countdown.reset(),
                        _ => {}
                    }
                }
                drop(state);
                self.update_ui();
                self.broadcast_widget_update();
            }
            _ => {}
        }
    }
}

impl Drop for ClockAtomicWidget {
    fn drop(&mut self) {
        cleanup_state(self.span_group.as_deref(), &self.shared_state);
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for ClockAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("clock atomic widget: received personalization status");
        let status = message.0;
        let timezone = status.timezone.as_ref().map(|t| t.to_string());
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride {
            timezone,
            locale,
            time_format: Some(status.time_format),
            date_format: Some(status.date_format),
        };
        *self.personalization.borrow_mut() = override_data;
        self.update_ui();
        self.broadcast_widget_update();
    }
}
