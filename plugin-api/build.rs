use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let glyph_list_path = Path::new(&manifest_dir).join("..").join("resources").join("glyph-list.txt");
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("nerd_font_map_generated.rs");

    println!("cargo:rerun-if-changed={}", glyph_list_path.display());

    let content = fs::read_to_string(&glyph_list_path).unwrap_or_default();
    let mut entries = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        let hex = parts[0];
        let prefix = parts[1];
        let name_words = &parts[2..];
        let name = name_words.join("-");
        let full_name = format!("nf-{}-{}", prefix, name);

        let codepoint = match u32::from_str_radix(hex, 16) {
            Ok(cp) => cp,
            Err(_) => continue,
        };

        entries.push((full_name, codepoint));
    }

    let mut output = String::new();
    output.push_str("use std::collections::HashMap;\n");
    output.push_str("use std::sync::OnceLock;\n\n");
    output.push_str("static NERD_FONT_MAP: OnceLock<HashMap<&'static str, u32>> = OnceLock::new();\n\n");
    output.push_str("fn build_map() -> HashMap<&'static str, u32> {\n");
    output.push_str("    HashMap::from([\n");

    for (name, cp) in &entries {
        output.push_str(&format!("        (\"{}\", {:#x}),\n", name, cp));
    }

    output.push_str("    ])\n");
    output.push_str("}\n\n");
    output.push_str("/// Resolve a Nerd Font icon name (e.g. `\"nf-fa-gamepad\"`) to its Unicode codepoint.\n");
    output.push_str("pub fn resolve(name: &str) -> Option<char> {\n");
    output.push_str("    let map = NERD_FONT_MAP.get_or_init(build_map);\n");
    output.push_str("    map.get(name).copied().and_then(char::from_u32)\n");
    output.push_str("}\n");

    fs::write(&dest_path, output).unwrap();
}
