pub mod list_web_instances;
pub mod render;
pub mod serve_instance_page;
pub mod web_instance_info;

pub use list_web_instances::list_web_instances;
pub use render::render_all_widgets_html;
pub use render::render_single_widget_html;
pub use serve_instance_page::serve_instance_page;
pub use web_instance_info::WebInstanceInfo;
