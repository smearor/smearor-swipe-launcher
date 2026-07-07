/// Encryption type of a WLAN access point.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WifiSecurity {
    /// Open network, no encryption.
    #[default]
    Open,
    /// WEP encryption (legacy).
    Wep,
    /// WPA/WPA2 encryption.
    Wpa,
    /// WPA3 encryption.
    Wpa3,
    /// Unknown or mixed encryption.
    Unknown,
}
