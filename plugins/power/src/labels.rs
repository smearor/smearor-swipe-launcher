use smearor_swipe_launcher_plugin_api::Locale;

/// Power widget labels that can be localized.
#[derive(Copy, Clone, Debug)]
pub enum PowerLabel {
    /// Label for the shutdown action.
    Shutdown,
    /// Label for the reboot action.
    Reboot,
    /// Label for the suspend action.
    Suspend,
    /// Label for the hibernate action.
    Hibernate,
    /// Label for the cancel action.
    Cancel,
    /// Label for the shutting down countdown, with a seconds placeholder.
    ShuttingDown,
    /// Label for the lock screen action.
    Lock,
    /// Label for the logout action.
    Logout,
    /// Label for the standby action.
    Standby,
    /// Label for the firmware reboot action.
    Firmware,
}

impl PowerLabel {
    /// Returns a localized label for the given key and locale.
    /// Falls back to English when the locale is not supported.
    pub fn localized_label(&self, locale: Locale) -> String {
        match locale {
            Locale::DeDe => self.german(),
            Locale::FrFr => self.french(),
            Locale::ItIt => self.italian(),
            Locale::EsEs => self.spanish(),
            _ => self.english(),
        }
    }

    fn english(&self) -> String {
        match self {
            PowerLabel::Shutdown => "Shutdown".to_string(),
            PowerLabel::Reboot => "Reboot".to_string(),
            PowerLabel::Suspend => "Suspend".to_string(),
            PowerLabel::Hibernate => "Hibernate".to_string(),
            PowerLabel::Cancel => "Cancel".to_string(),
            PowerLabel::ShuttingDown => "Shutting down in {n}s".to_string(),
            PowerLabel::Lock => "Lock".to_string(),
            PowerLabel::Logout => "Logout".to_string(),
            PowerLabel::Standby => "Standby".to_string(),
            PowerLabel::Firmware => "Firmware".to_string(),
        }
    }

    fn german(&self) -> String {
        match self {
            PowerLabel::Shutdown => "Herunterfahren".to_string(),
            PowerLabel::Reboot => "Neustart".to_string(),
            PowerLabel::Suspend => "Ruhezustand".to_string(),
            PowerLabel::Hibernate => "Tiefschlaf".to_string(),
            PowerLabel::Cancel => "Abbrechen".to_string(),
            PowerLabel::ShuttingDown => "Herunterfahren in {n}s".to_string(),
            PowerLabel::Lock => "Sperren".to_string(),
            PowerLabel::Logout => "Abmelden".to_string(),
            PowerLabel::Standby => "Bereitschaft".to_string(),
            PowerLabel::Firmware => "Firmware".to_string(),
        }
    }

    fn french(&self) -> String {
        match self {
            PowerLabel::Shutdown => "Arr\u{ea}ter".to_string(),
            PowerLabel::Reboot => "Red\u{e9}marrer".to_string(),
            PowerLabel::Suspend => "Mettre en veille".to_string(),
            PowerLabel::Hibernate => "Hibernation".to_string(),
            PowerLabel::Cancel => "Annuler".to_string(),
            PowerLabel::ShuttingDown => "Arr\u{ea}t dans {n}s".to_string(),
            PowerLabel::Lock => "Verrouiller".to_string(),
            PowerLabel::Logout => "D\u{e9}connexion".to_string(),
            PowerLabel::Standby => "Veille".to_string(),
            PowerLabel::Firmware => "Firmware".to_string(),
        }
    }

    fn spanish(&self) -> String {
        match self {
            PowerLabel::Shutdown => "Apagar".to_string(),
            PowerLabel::Reboot => "Reiniciar".to_string(),
            PowerLabel::Suspend => "Suspender".to_string(),
            PowerLabel::Hibernate => "Hibernar".to_string(),
            PowerLabel::Cancel => "Cancelar".to_string(),
            PowerLabel::ShuttingDown => "Apagando en {n}s".to_string(),
            PowerLabel::Lock => "Bloquear".to_string(),
            PowerLabel::Logout => "Cerrar sesi\u{f3}n".to_string(),
            PowerLabel::Standby => "En espera".to_string(),
            PowerLabel::Firmware => "Firmware".to_string(),
        }
    }

    fn italian(&self) -> String {
        match self {
            PowerLabel::Shutdown => "Spegni".to_string(),
            PowerLabel::Reboot => "Riavvia".to_string(),
            PowerLabel::Suspend => "Sospendi".to_string(),
            PowerLabel::Hibernate => "Iberna".to_string(),
            PowerLabel::Cancel => "Annulla".to_string(),
            PowerLabel::ShuttingDown => "Spegnimento in {n}s".to_string(),
            PowerLabel::Lock => "Blocca".to_string(),
            PowerLabel::Logout => "Esci".to_string(),
            PowerLabel::Standby => "Standby".to_string(),
            PowerLabel::Firmware => "Firmware".to_string(),
        }
    }

    /// Formats a label with a seconds value, replacing `{n}` with the number.
    pub fn format_with_seconds(label: &str, seconds: u32) -> String {
        label.replace("{n}", &seconds.to_string())
    }
}
