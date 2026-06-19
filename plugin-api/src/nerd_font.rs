// Nerd Font icon name resolution.
//
// Maps human-readable icon names like `nf-fa-gamepad` to their Unicode
// codepoints using the glyph list compiled into the binary.

include!(concat!(env!("OUT_DIR"), "/nerd_font_map_generated.rs"));
