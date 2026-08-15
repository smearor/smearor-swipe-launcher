use smearor_swipe_launcher_plugin_api::Locale;

/// Notifications widget labels that can be localized.
#[derive(Copy, Clone, Debug)]
pub enum NotificationLabel {
    /// "Notifications" header label.
    Notifications,
    /// "No notifications" empty state label.
    NoNotifications,
    /// "Do Not Disturb" label.
    DoNotDisturb,
    /// "DND" short label.
    Dnd,
    /// "Just now" relative time label.
    JustNow,
    /// "X minutes ago" relative time label.
    MinutesAgo,
    /// "X hours ago" relative time label.
    HoursAgo,
    /// "X days ago" relative time label.
    DaysAgo,
}

impl NotificationLabel {
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
            Self::Notifications => "Notifications",
            Self::NoNotifications => "No notifications",
            Self::DoNotDisturb => "Do Not Disturb",
            Self::Dnd => "DND",
            Self::JustNow => "Just now",
            Self::MinutesAgo => "min ago",
            Self::HoursAgo => "h ago",
            Self::DaysAgo => "d ago",
        }
    }

    fn german(&self) -> &'static str {
        match self {
            Self::Notifications => "Benachrichtigungen",
            Self::NoNotifications => "Keine Benachrichtigungen",
            Self::DoNotDisturb => "Nicht st\u{f6}ren",
            Self::Dnd => "NS",
            Self::JustNow => "Gerade eben",
            Self::MinutesAgo => "min",
            Self::HoursAgo => "Std",
            Self::DaysAgo => "T",
        }
    }

    fn french(&self) -> &'static str {
        match self {
            Self::Notifications => "Notifications",
            Self::NoNotifications => "Aucune notification",
            Self::DoNotDisturb => "Ne pas d\u{e9}ranger",
            Self::Dnd => "NPD",
            Self::JustNow => "\u{c0} l'instant",
            Self::MinutesAgo => "min",
            Self::HoursAgo => "h",
            Self::DaysAgo => "j",
        }
    }

    fn spanish(&self) -> &'static str {
        match self {
            Self::Notifications => "Notificaciones",
            Self::NoNotifications => "Sin notificaciones",
            Self::DoNotDisturb => "No molestar",
            Self::Dnd => "NM",
            Self::JustNow => "Ahora mismo",
            Self::MinutesAgo => "min",
            Self::HoursAgo => "h",
            Self::DaysAgo => "d",
        }
    }

    fn italian(&self) -> &'static str {
        match self {
            Self::Notifications => "Notifiche",
            Self::NoNotifications => "Nessuna notifica",
            Self::DoNotDisturb => "Non disturbare",
            Self::Dnd => "ND",
            Self::JustNow => "Adesso",
            Self::MinutesAgo => "min",
            Self::HoursAgo => "h",
            Self::DaysAgo => "g",
        }
    }
}
