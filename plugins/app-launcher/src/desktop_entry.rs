pub(crate) struct DesktopEntry {
    pub(crate) name: String,
    pub(crate) icon: String,
}

impl DesktopEntry {
    pub(crate) fn parse(path: &str) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        let mut name = None;
        let mut icon = None;
        let mut in_desktop_entry = false;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                in_desktop_entry = line == "[Desktop Entry]";
                continue;
            }
            if !in_desktop_entry {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                match key.trim() {
                    "Name" => name = Some(val.trim().to_string()),
                    "Icon" => icon = Some(val.trim().to_string()),
                    _ => {}
                }
            }
        }

        Some(DesktopEntry {
            name: name.unwrap_or_else(|| "Unknown App".to_string()),
            icon: icon.unwrap_or_else(|| "system-run".to_string()),
        })
    }
}
