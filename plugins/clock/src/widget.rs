use crate::clock::Clock;
use crate::config::ClockConfig;
use gtk4::Box as GtkBox;
use gtk4::GestureClick;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
use serde_json;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::Plugin;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct CyberLabels {
    hour_prev: Label,
    hour_curr: Label,
    hour_next: Label,
    min_prev: Label,
    min_curr: Label,
    min_next: Label,
    sec_prev: Label,
    sec_curr: Label,
    sec_next: Label,
    date_label: Option<Label>,
}

pub(crate) struct ClockWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: ClockConfig,
    pub(crate) clock: Arc<Clock>,
    pub(crate) labels: Arc<RwLock<Option<CyberLabels>>>,
    pub(crate) time_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<()>>,
}

impl ClockWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let clock_config: ClockConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        Ok(ClockWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: clock_config.clone(),
            clock: Arc::new(Clock::new(clock_config)),
            labels: Arc::new(RwLock::new(None)),
            time_receiver: None,
        })
    }

    fn create_time_column(css_prefix: &str) -> (GtkBox, (Label, Label, Label)) {
        let col_box = GtkBox::builder().orientation(Orientation::Vertical).spacing(2).build();

        let lbl_prev = Label::builder().css_classes([format!("cyber-clock-{}-prev", css_prefix)]).build();
        let lbl_curr = Label::builder().css_classes([format!("cyber-clock-{}-curr", css_prefix)]).build();
        let lbl_next = Label::builder().css_classes([format!("cyber-clock-{}-next", css_prefix)]).build();

        col_box.append(&lbl_prev);
        col_box.append(&lbl_curr);
        col_box.append(&lbl_next);

        (col_box, (lbl_prev, lbl_curr, lbl_next))
    }

    fn create_divider() -> Label {
        Label::builder()
            .label(":")
            .valign(gtk4::Align::Center)
            .css_classes(["cyber-clock-divider".to_string()])
            .build()
    }

    fn update_labels(labels: &CyberLabels, clock: &Clock) {
        let h = clock.get_hour() as i32;
        let m = clock.get_minute() as i32;
        let s = clock.get_second() as i32;

        labels.hour_prev.set_text(&format!("{:02}", (h - 1 + 24) % 24));
        labels.hour_curr.set_text(&format!("{:02}", h));
        labels.hour_next.set_text(&format!("{:02}", (h + 1) % 24));

        labels.min_prev.set_text(&format!("{:02}", (m - 1 + 60) % 60));
        labels.min_curr.set_text(&format!("{:02}", m));
        labels.min_next.set_text(&format!("{:02}", (m + 1) % 60));

        labels.sec_prev.set_text(&format!("{:02}", (s - 1 + 60) % 60));
        labels.sec_curr.set_text(&format!("{:02}", s));
        labels.sec_next.set_text(&format!("{:02}", (s + 1) % 60));

        if let Some(ref date_label) = labels.date_label {
            if let Some(date_str) = clock.get_current_time_2() {
                date_label.set_text(&date_str);
            }
        }
    }

    pub(crate) fn start_time_update(&mut self) {
        let (time_sender, time_receiver) = tokio::sync::mpsc::unbounded_channel::<()>();
        self.time_receiver = Some(time_receiver);

        thread::spawn(move || {
            loop {
                if time_sender.send(()).is_err() {
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }
        });

        if let Some(mut rx) = self.time_receiver.take() {
            let labels = self.labels.clone();
            let clock = self.clock.clone();
            MainContext::default().spawn_local(async move {
                while rx.recv().await.is_some() {
                    if let Ok(guard) = labels.read() {
                        if let Some(ref lbls) = guard.as_ref() {
                            Self::update_labels(lbls, &clock);
                        }
                    }
                }
            });
        }
    }
}

impl MessageHandler<FfiEnvelope> for ClockWidget {
    fn handle_message(&self, _message: FfiEnvelope, _sender_id: &str) {}
}

impl AcceptTopic<FfiEnvelope> for ClockWidget {
    fn accept_topic(&self, _topic: &str) -> bool {
        false
    }
}

impl MessageBroadcaster for ClockWidget {}

impl PluginMetaGetter for ClockWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for ClockWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Plugin for ClockWidget {}

impl WidgetBuilder for ClockWidget {
    fn build_widget(&mut self) -> Widget {
        let outer_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .css_classes(["cyber-clock-main".to_string()])
            .build();

        let time_box = GtkBox::builder().orientation(Orientation::Horizontal).spacing(self.config.spacing).build();

        if let Some(width) = self.config.width {
            outer_box.set_width_request(width);
        }

        let (box_hour, hour_labels) = Self::create_time_column("hour");
        let (box_min, min_labels) = Self::create_time_column("minute");
        let (box_sec, sec_labels) = Self::create_time_column("second");

        let divider1 = Self::create_divider();
        let divider2 = Self::create_divider();

        time_box.append(&box_hour);
        time_box.append(&divider1);
        time_box.append(&box_min);
        time_box.append(&divider2);
        time_box.append(&box_sec);

        outer_box.append(&time_box);

        let mut date_label: Option<Label> = None;
        if self.config.format_2.is_some() {
            let date = Label::builder().css_classes(["cyber-clock-date".to_string()]).build();
            outer_box.append(&date);
            date_label = Some(date);
        }

        let labels = CyberLabels {
            hour_prev: hour_labels.0,
            hour_curr: hour_labels.1,
            hour_next: hour_labels.2,
            min_prev: min_labels.0,
            min_curr: min_labels.1,
            min_next: min_labels.2,
            sec_prev: sec_labels.0,
            sec_curr: sec_labels.1,
            sec_next: sec_labels.2,
            date_label,
        };

        Self::update_labels(&labels, &self.clock);

        let click_topic = self.config.click_topic.clone();
        let click_payload = self.config.click_payload.clone();
        let message_broadcaster = self.get_broadcaster();
        let gesture = GestureClick::new();
        gesture.connect_released(move |_gesture, _, _, _| {
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                let payload_str = payload.to_string();
                message_broadcaster.broadcast_string(&topic, &payload_str);
            }
        });
        outer_box.add_controller(gesture);

        *self.labels.write().unwrap() = Some(labels);
        self.start_time_update();
        outer_box.upcast::<Widget>()
    }
}
