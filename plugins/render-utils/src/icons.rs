/// Resolve a Nerd Font icon name (e.g. `nf-weather-day_sunny`) to its
/// Unicode codepoint character by looking it up in the `nerd_gtk_icons`
/// codepoint map.
///
/// The name is normalized to the GTK symbolic icon name format
/// (kebab-case, `-symbolic` suffix) before lookup.
pub fn resolve_icon_codepoint(icon_name: &str) -> Option<char> {
    let normalized = icon_name.replace('_', "-").to_lowercase();
    let with_suffix = if normalized.ends_with("-symbolic") {
        normalized
    } else {
        format!("{}-symbolic", normalized)
    };
    nerd_gtk_icons::codepoint_map::ICONS
        .entries()
        .find(|(_, name)| **name == with_suffix)
        .map(|(c, _)| *c)
}
