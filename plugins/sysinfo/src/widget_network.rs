use crate::config::NetworkDisplayMode;
use crate::config::NetworkWidgetConfig;
use crate::shared::build_gauge_container;
use crate::shared::build_icon_image;
use crate::shared::draw_network_gauge;
use crate::shared::format_bytes;
use crate::shared::gauge_color;
use glib::object::Cast;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::DrawingArea;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::DrawingAreaExtManual;
use gtk4::prelude::WidgetExt;
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
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_sysinfo_model::NetworkStatusMessage;
use smearor_sysinfo_model::TOPIC_NETWORK;
use std::cell::RefCell;
use std::rc::Rc;

pub struct NetworkWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: NetworkWidgetConfig,
    pub current_download: Rc<RefCell<f32>>,
    pub current_upload: Rc<RefCell<f32>>,
    pub container: Rc<RefCell<Option<GtkBox>>>,
    pub received_label: Rc<RefCell<Option<Label>>>,
    pub transmitted_label: Rc<RefCell<Option<Label>>>,
    pub icon_image: Rc<RefCell<Option<Image>>>,
    pub gauge: Rc<RefCell<Option<DrawingArea>>>,
}

impl NetworkWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: NetworkWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        Ok(Self {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            current_download: Rc::new(RefCell::new(0.0)),
            current_upload: Rc::new(RefCell::new(0.0)),
            container: Rc::new(RefCell::new(None)),
            received_label: Rc::new(RefCell::new(None)),
            transmitted_label: Rc::new(RefCell::new(None)),
            icon_image: Rc::new(RefCell::new(None)),
            gauge: Rc::new(RefCell::new(None)),
        })
    }

    fn update_ui(&self, message: &NetworkStatusMessage) {
        let received_label = self.received_label.clone();
        let transmitted_label = self.transmitted_label.clone();
        let gauge = self.gauge.clone();
        let config = self.config.clone();

        let download_pct = if config.max_download > 0.0 {
            (message.received_bytes_per_second as f64 / config.max_download * 100.0).min(100.0) as f32
        } else {
            0.0
        };
        let upload_pct = if config.max_upload > 0.0 {
            (message.transmitted_bytes_per_second as f64 / config.max_upload * 100.0).min(100.0) as f32
        } else {
            0.0
        };

        *self.current_download.borrow_mut() = download_pct;
        *self.current_upload.borrow_mut() = upload_pct;

        let message_inner = message.clone();
        MainContext::default().spawn_local(async move {
            if let Some(ref label) = *received_label.borrow() {
                if config.show_received {
                    label.set_text(&format!("down: {}/s", format_bytes(message_inner.received_bytes_per_second)));
                } else {
                    label.set_text("");
                }
            }
            if let Some(ref label) = *transmitted_label.borrow() {
                if config.show_transmitted {
                    label.set_text(&format!("up: {}/s", format_bytes(message_inner.transmitted_bytes_per_second)));
                } else {
                    label.set_text("");
                }
            }
            if let Some(ref gauge_widget) = *gauge.borrow() {
                gauge_widget.queue_draw();
            }
        });
    }
}

impl MessageHandler<NetworkStatusMessage> for NetworkWidget {
    fn handle_message(&self, message: NetworkStatusMessage, _sender_id: &str) {
        self.update_ui(&message);
    }
}

impl AcceptTopic<FfiEnvelope> for NetworkWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_NETWORK
    }
}

impl MessageBroadcaster for NetworkWidget {}

impl PluginMetaGetter for NetworkWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for NetworkWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Plugin for NetworkWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == NetworkStatusMessage::TYPE_ID {
                    MessageHandler::<NetworkStatusMessage>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for NetworkWidget {
    fn build_widget(&mut self) -> Widget {
        if self.config.display_mode == NetworkDisplayMode::Gauge {
            return self.build_gauge_widget();
        }

        let container = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .css_classes(["sysinfo-network".to_string()])
            .build();

        let mut icon_image = None;
        if self.config.show_icon {
            if let Some(ref icon) = self.config.icon {
                let image = build_icon_image(icon, self.config.icon_size);
                image.add_css_class("sysinfo-icon");
                container.append(&image);
                icon_image = Some(image);
            }
        }

        let content_area = GtkBox::builder().orientation(Orientation::Vertical).spacing(4).build();
        container.append(&content_area);

        let mut received_label = None;
        let mut transmitted_label = None;

        if self.config.show_received {
            let label = Label::builder().label("down: 0 B/s").build();
            content_area.append(&label);
            received_label = Some(label);
        }
        if self.config.show_transmitted {
            let label = Label::builder().label("up: 0 B/s").build();
            content_area.append(&label);
            transmitted_label = Some(label);
        }

        *self.container.borrow_mut() = Some(container.clone());
        *self.received_label.borrow_mut() = received_label;
        *self.transmitted_label.borrow_mut() = transmitted_label;
        *self.icon_image.borrow_mut() = icon_image;

        container.upcast::<Widget>()
    }
}

impl NetworkWidget {
    fn build_gauge_widget(&mut self) -> Widget {
        let gauge_widget = build_gauge_container(120, "sysinfo-gauge");
        let content_box = gauge_widget.content_box;

        let mut icon_image = None;
        if self.config.show_icon {
            if let Some(ref icon) = self.config.icon {
                let image = build_icon_image(icon, self.config.icon_size);
                image.add_css_class("sysinfo-icon");
                content_box.append(&image);
                icon_image = Some(image);
            }
        }

        let mut received_label = None;
        let mut transmitted_label = None;

        if self.config.show_received {
            let label = Label::builder()
                .css_classes(["sysinfo-details".to_string()])
                .label("down: 0 B/s")
                .halign(Align::Center)
                .build();
            content_box.append(&label);
            received_label = Some(label);
        }
        if self.config.show_transmitted {
            let label = Label::builder()
                .css_classes(["sysinfo-details".to_string()])
                .label("up: 0 B/s")
                .halign(Align::Center)
                .build();
            content_box.append(&label);
            transmitted_label = Some(label);
        }

        let drawing_area = gauge_widget.drawing_area;
        let current_download = self.current_download.clone();
        let current_upload = self.current_upload.clone();
        drawing_area.set_draw_func(move |_area, context, width, height| {
            let download_value = *current_download.borrow();
            let upload_value = *current_upload.borrow();
            draw_network_gauge(
                context,
                width,
                height,
                download_value,
                upload_value,
                gauge_color(download_value, 70.0, 90.0),
                gauge_color(upload_value, 70.0, 90.0),
            );
        });

        *self.received_label.borrow_mut() = received_label;
        *self.transmitted_label.borrow_mut() = transmitted_label;
        *self.icon_image.borrow_mut() = icon_image;
        *self.gauge.borrow_mut() = Some(drawing_area);

        gauge_widget.outer_widget
    }
}
