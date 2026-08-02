use smearor_swipe_launcher_plugin_api::Locale;

/// Network widget labels that can be localized.
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub enum NetworkLabel {
    /// Connection state: connected.
    Connected,
    /// Connection state: disconnected.
    Disconnected,
    /// Signal label.
    Signal,
    /// Signal strength label.
    Strength,
    /// Download direction label.
    Download,
    /// Upload direction label.
    Upload,
    /// WiFi label.
    WiFi,
    /// No WiFi available.
    NoWiFi,
    /// No Ethernet available.
    NoEthernet,
    /// No VPN configured.
    NoVpn,
    /// VPN active state.
    Active,
    /// VPN inactive state.
    Inactive,
    /// Airplane mode label.
    AirplaneMode,
    /// QR code label.
    QrCode,
    /// Networks count label.
    Networks,
    /// Unknown label.
    Unknown,
}

impl NetworkLabel {
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
            Self::Connected => "Connected",
            Self::Disconnected => "Disconnected",
            Self::Signal => "Signal",
            Self::Strength => "Strength",
            Self::Download => "Download",
            Self::Upload => "Upload",
            Self::WiFi => "WiFi",
            Self::NoWiFi => "WiFi Off",
            Self::NoEthernet => "No Ethernet",
            Self::NoVpn => "No VPN",
            Self::Active => "Active",
            Self::Inactive => "Inactive",
            Self::AirplaneMode => "Airplane Mode",
            Self::QrCode => "QR Code",
            Self::Networks => "networks",
            Self::Unknown => "Unknown",
        }
    }

    fn german(&self) -> &'static str {
        match self {
            Self::Connected => "Verbunden",
            Self::Disconnected => "Getrennt",
            Self::Signal => "Signal",
            Self::Strength => "St\u{e4}rke",
            Self::Download => "Download",
            Self::Upload => "Upload",
            Self::WiFi => "WLAN",
            Self::NoWiFi => "WLAN Aus",
            Self::NoEthernet => "Kein Ethernet",
            Self::NoVpn => "Kein VPN",
            Self::Active => "Aktiv",
            Self::Inactive => "Inaktiv",
            Self::AirplaneMode => "Flugmodus",
            Self::QrCode => "QR-Code",
            Self::Networks => "Netzwerke",
            Self::Unknown => "Unbekannt",
        }
    }

    fn french(&self) -> &'static str {
        match self {
            Self::Connected => "Connect\u{e9}",
            Self::Disconnected => "D\u{e9}connect\u{e9}",
            Self::Signal => "Signal",
            Self::Strength => "Force",
            Self::Download => "T\u{e9}l\u{e9}chargement",
            Self::Upload => "Envoi",
            Self::WiFi => "WiFi",
            Self::NoWiFi => "WiFi \u{e9}teint",
            Self::NoEthernet => "Pas d'Ethernet",
            Self::NoVpn => "Pas de VPN",
            Self::Active => "Actif",
            Self::Inactive => "Inactif",
            Self::AirplaneMode => "Mode avion",
            Self::QrCode => "QR Code",
            Self::Networks => "r\u{e9}seaux",
            Self::Unknown => "Inconnu",
        }
    }

    fn spanish(&self) -> &'static str {
        match self {
            Self::Connected => "Conectado",
            Self::Disconnected => "Desconectado",
            Self::Signal => "Se\u{f1}al",
            Self::Strength => "Intensidad",
            Self::Download => "Descarga",
            Self::Upload => "Subida",
            Self::WiFi => "WiFi",
            Self::NoWiFi => "WiFi Apagado",
            Self::NoEthernet => "Sin Ethernet",
            Self::NoVpn => "Sin VPN",
            Self::Active => "Activo",
            Self::Inactive => "Inactivo",
            Self::AirplaneMode => "Modo avi\u{f3}n",
            Self::QrCode => "C\u{f3}digo QR",
            Self::Networks => "redes",
            Self::Unknown => "Desconocido",
        }
    }

    fn italian(&self) -> &'static str {
        match self {
            Self::Connected => "Connesso",
            Self::Disconnected => "Disconnesso",
            Self::Signal => "Segnale",
            Self::Strength => "Intensit\u{e0}",
            Self::Download => "Download",
            Self::Upload => "Upload",
            Self::WiFi => "WiFi",
            Self::NoWiFi => "WiFi Spento",
            Self::NoEthernet => "Nessun Ethernet",
            Self::NoVpn => "Nessun VPN",
            Self::Active => "Attivo",
            Self::Inactive => "Inattivo",
            Self::AirplaneMode => "Modalit\u{e0} aereo",
            Self::QrCode => "Codice QR",
            Self::Networks => "reti",
            Self::Unknown => "Sconosciuto",
        }
    }
}
