/// Internal command channel for device operations.
#[derive(Clone, Debug)]
pub enum DeviceCommand {
    SetBrightness(u8),
    ClearAllButtons,
    ClearButton(u8),
    SetButtonImage(u8, u32, u32, Vec<u8>),
    Reset,
}
