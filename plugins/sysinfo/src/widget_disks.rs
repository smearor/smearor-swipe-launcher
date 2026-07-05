use crate::config::DiskDisplayMode;
use crate::config::DisksWidgetConfig;
use crate::shared::build_icon_image;
use crate::shared::format_bytes;
use glib::object::Cast;
use gtk4::Box as GtkBox;
use gtk4::Image;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
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
use smearor_sysinfo_model::DisksStatusMessage;
use smearor_sysinfo_model::TOPIC_DISKS;
use std::cell::RefCell;
use std::rc::Rc;

pub struct DisksWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: DisksWidgetConfig,
    pub container: Rc<RefCell<Option<GtkBox>>>,
    pub content_area: Rc<RefCell<Option<GtkBox>>>,
    pub throughput_label: Rc<RefCell<Option<Label>>>,
    pub icon_image: Rc<RefCell<Option<Image>>>,
}

impl DisksWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: DisksWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        Ok(Self {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            container: Rc::new(RefCell::new(None)),
            content_area: Rc::new(RefCell::new(None)),
            throughput_label: Rc::new(RefCell::new(None)),
            icon_image: Rc::new(RefCell::new(None)),
        })
    }

    fn update_ui(&self, message: &DisksStatusMessage) {
        let content_area = self.content_area.clone();
        let throughput_label = self.throughput_label.clone();
        let config = self.config.clone();
        let message_inner = message.clone();

        MainContext::default().spawn_local(async move {
            if let Some(ref content) = *content_area.borrow() {
                while let Some(child) = content.first_child() {
                    content.remove(&child);
                }

                let mut count = 0;
                for mount in message_inner.mounts.iter() {
                    if count >= config.max_mount_points {
                        break;
                    }
                    let mount_point = mount.mount_point.to_string();
                    if !config.include_mount_points.is_empty() && !config.include_mount_points.contains(&mount_point) {
                        continue;
                    }
                    if config.display_mode == DiskDisplayMode::RootOnly && mount_point != "/" {
                        continue;
                    }

                    let row = GtkBox::builder().orientation(Orientation::Horizontal).spacing(4).build();
                    let label = Label::builder().label(&format!("{}: {:.0}%", mount_point, mount.usage)).build();
                    row.append(&label);
                    let bar = LevelBar::builder()
                        .min_value(0.0)
                        .max_value(100.0)
                        .value(mount.usage as f64)
                        .width_request(80)
                        .build();
                    row.append(&bar);
                    content.append(&row);
                    count += 1;
                }

                if count == 0 {
                    let placeholder = Label::builder().label("no disk data").build();
                    content.append(&placeholder);
                }

                if config.show_throughput {
                    let throughput = Label::builder()
                        .label(&format!(
                            "R: {}/s | W: {}/s",
                            format_bytes(message_inner.read_bytes_per_second),
                            format_bytes(message_inner.write_bytes_per_second)
                        ))
                        .build();
                    content.append(&throughput);
                }
            }

            if let Some(ref label) = *throughput_label.borrow() {
                label.set_text(&format!(
                    "R: {}/s | W: {}/s",
                    format_bytes(message_inner.read_bytes_per_second),
                    format_bytes(message_inner.write_bytes_per_second)
                ));
            }
        });
    }
}

impl MessageHandler<DisksStatusMessage> for DisksWidget {
    fn handle_message(&self, message: DisksStatusMessage, _sender_id: &str) {
        self.update_ui(&message);
    }
}

impl AcceptTopic<FfiEnvelope> for DisksWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_DISKS
    }
}

impl MessageBroadcaster for DisksWidget {}

impl PluginMetaGetter for DisksWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for DisksWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Plugin for DisksWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == DisksStatusMessage::TYPE_ID {
                    MessageHandler::<DisksStatusMessage>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for DisksWidget {
    fn build_widget(&mut self) -> Widget {
        let container = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .css_classes(["sysinfo-disks".to_string()])
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

        let mut throughput_label = None;
        if self.config.show_throughput {
            let label = Label::builder().label("R: 0 B/s | W: 0 B/s").build();
            content_area.append(&label);
            throughput_label = Some(label);
        }

        let placeholder = Label::builder().label("waiting for disk data...").build();
        content_area.append(&placeholder);

        *self.container.borrow_mut() = Some(container.clone());
        *self.content_area.borrow_mut() = Some(content_area);
        *self.throughput_label.borrow_mut() = throughput_label;
        *self.icon_image.borrow_mut() = icon_image;

        container.upcast::<Widget>()
    }
}
