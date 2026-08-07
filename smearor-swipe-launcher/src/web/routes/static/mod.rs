pub mod serve_css;
pub mod serve_js;
pub mod serve_nerdfont_css;
pub mod serve_nerdfont_woff2;

pub use serve_css::serve_static_css;
pub use serve_js::serve_static_js;
pub use serve_nerdfont_css::serve_static_nerdfont_css;
pub use serve_nerdfont_woff2::serve_static_nerdfont_woff2;
