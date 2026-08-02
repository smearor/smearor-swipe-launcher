use smearor_swipe_launcher_plugin_api::Locale;

/// Sysinfo widget labels that can be localized.
#[derive(Copy, Clone, Debug)]
pub enum SysinfoLabel {
    /// CPU label.
    Cpu,
    /// Memory label.
    Memory,
    /// Disk label.
    Disk,
    /// Network label.
    Network,
    /// Temperature label.
    Temperature,
    /// Upload label.
    Upload,
    /// Download label.
    Download,
    /// Battery label.
    Battery,
    /// Uptime label.
    Uptime,
    /// Load average label.
    Load,
}

impl SysinfoLabel {
    /// Returns a localized label for the given key and locale.
    /// Falls back to English when the locale is not supported.
    pub fn localized_label(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::DeDe => self.german(),
            Locale::FrFr => self.french(),
            Locale::ItIt => self.italian(),
            Locale::EsEs => self.spanish(),
            _ => self.english(),
        }
    }

    fn english(&self) -> &'static str {
        match self {
            SysinfoLabel::Cpu => "CPU",
            SysinfoLabel::Memory => "Memory",
            SysinfoLabel::Disk => "Disk",
            SysinfoLabel::Network => "Network",
            SysinfoLabel::Temperature => "Temp",
            SysinfoLabel::Upload => "Upload",
            SysinfoLabel::Download => "Download",
            SysinfoLabel::Battery => "Battery",
            SysinfoLabel::Uptime => "Uptime",
            SysinfoLabel::Load => "Load",
        }
    }

    fn german(&self) -> &'static str {
        match self {
            SysinfoLabel::Cpu => "Prozessor",
            SysinfoLabel::Memory => "Arbeitsspeicher",
            SysinfoLabel::Disk => "Festplatte",
            SysinfoLabel::Network => "Netzwerk",
            SysinfoLabel::Temperature => "Temp",
            SysinfoLabel::Upload => "Upload",
            SysinfoLabel::Download => "Download",
            SysinfoLabel::Battery => "Akku",
            SysinfoLabel::Uptime => "Laufzeit",
            SysinfoLabel::Load => "Last",
        }
    }

    fn french(&self) -> &'static str {
        match self {
            SysinfoLabel::Cpu => "Processeur",
            SysinfoLabel::Memory => "M\u{e9}moire",
            SysinfoLabel::Disk => "Disque",
            SysinfoLabel::Network => "R\u{e9}seau",
            SysinfoLabel::Temperature => "Temp",
            SysinfoLabel::Upload => "Envoi",
            SysinfoLabel::Download => "R\u{e9}ception",
            SysinfoLabel::Battery => "Batterie",
            SysinfoLabel::Uptime => "Disponibilit\u{e9}",
            SysinfoLabel::Load => "Charge",
        }
    }

    fn spanish(&self) -> &'static str {
        match self {
            SysinfoLabel::Cpu => "Procesador",
            SysinfoLabel::Memory => "Memoria",
            SysinfoLabel::Disk => "Disco",
            SysinfoLabel::Network => "Red",
            SysinfoLabel::Temperature => "Temp",
            SysinfoLabel::Upload => "Subida",
            SysinfoLabel::Download => "Descarga",
            SysinfoLabel::Battery => "Bater\u{ed}a",
            SysinfoLabel::Uptime => "Tiempo activo",
            SysinfoLabel::Load => "Carga",
        }
    }

    fn italian(&self) -> &'static str {
        match self {
            SysinfoLabel::Cpu => "Processore",
            SysinfoLabel::Memory => "Memoria",
            SysinfoLabel::Disk => "Disco",
            SysinfoLabel::Network => "Rete",
            SysinfoLabel::Temperature => "Temp",
            SysinfoLabel::Upload => "Upload",
            SysinfoLabel::Download => "Download",
            SysinfoLabel::Battery => "Batteria",
            SysinfoLabel::Uptime => "Tempo attivo",
            SysinfoLabel::Load => "Carico",
        }
    }
}
