use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::system::AddMasterDispatchMessage;
use crate::dispatch::system::BringActiveToTopDispatchMessage;
use crate::dispatch::system::CustomDispatchMessageStabby;
use crate::dispatch::system::ExitDispatchMessage;
use crate::dispatch::system::ForceRendererReloadDispatchMessage;
use crate::dispatch::system::GlobalDispatchMessageStabby;
use crate::dispatch::system::LockGroupsDispatchMessage;
use crate::dispatch::system::MoveOutOfGroupDispatchMessage;
use crate::dispatch::system::OrientationBottomDispatchMessage;
use crate::dispatch::system::OrientationCenterDispatchMessage;
use crate::dispatch::system::OrientationLeftDispatchMessage;
use crate::dispatch::system::OrientationNextDispatchMessage;
use crate::dispatch::system::OrientationPrevDispatchMessage;
use crate::dispatch::system::OrientationRightDispatchMessage;
use crate::dispatch::system::OrientationTopDispatchMessage;
use crate::dispatch::system::PassDispatchMessage;
use crate::dispatch::system::RemoveMasterDispatchMessage;
use crate::dispatch::system::SetCursorDispatchMessageStabby;

/// Kind for system-related dispatch commands.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemDispatchKind {
    #[default]
    AddMaster,
    BringActiveToTop,
    Custom,
    Exit,
    ForceRendererReload,
    Global,
    LockGroups,
    MoveOutOfGroup,
    OrientationBottom,
    OrientationCenter,
    OrientationLeft,
    OrientationNext,
    OrientationPrev,
    OrientationRight,
    OrientationTop,
    Pass,
    RemoveMaster,
    SetCursor,
}

/// System-related dispatch options (master, orientation, misc).
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SystemDispatchOps {
    pub add_master: stabby::option::Option<AddMasterDispatchMessage>,
    pub bring_active_to_top: stabby::option::Option<BringActiveToTopDispatchMessage>,
    pub custom: stabby::option::Option<CustomDispatchMessageStabby>,
    pub exit: stabby::option::Option<ExitDispatchMessage>,
    pub force_renderer_reload: stabby::option::Option<ForceRendererReloadDispatchMessage>,
    pub global: stabby::option::Option<GlobalDispatchMessageStabby>,
    pub lock_groups: stabby::option::Option<LockGroupsDispatchMessage>,
    pub move_out_of_group: stabby::option::Option<MoveOutOfGroupDispatchMessage>,
    pub orientation_bottom: stabby::option::Option<OrientationBottomDispatchMessage>,
    pub orientation_center: stabby::option::Option<OrientationCenterDispatchMessage>,
    pub orientation_left: stabby::option::Option<OrientationLeftDispatchMessage>,
    pub orientation_next: stabby::option::Option<OrientationNextDispatchMessage>,
    pub orientation_prev: stabby::option::Option<OrientationPrevDispatchMessage>,
    pub orientation_right: stabby::option::Option<OrientationRightDispatchMessage>,
    pub orientation_top: stabby::option::Option<OrientationTopDispatchMessage>,
    pub pass: stabby::option::Option<PassDispatchMessage>,
    pub remove_master: stabby::option::Option<RemoveMasterDispatchMessage>,
    pub set_cursor: stabby::option::Option<SetCursorDispatchMessageStabby>,
}

impl TypedMessage for SystemDispatchKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SystemDispatchKind");
}
