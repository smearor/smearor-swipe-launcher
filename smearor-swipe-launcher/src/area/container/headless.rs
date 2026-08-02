use crate::area::container::area::AreaContainer;
use crate::area::overlay::headless::HeadlessOverlay;

/// A no-op container for headless instances.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct HeadlessContainer;

impl AreaContainer for HeadlessContainer {
    type Overlay = HeadlessOverlay;

    fn append_overlay(&self, _overlay: &HeadlessOverlay) {}

    fn remove_overlay(&self, _overlay: &HeadlessOverlay) {}
}
