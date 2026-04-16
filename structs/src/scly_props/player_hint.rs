use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct PlayerHintStruct {
    #[auto_struct(expect = 15)]
    prop_count: u32,

    pub unknown1: u8,
    pub unknown2: u8,
    pub extend_target_distance: u8,
    pub unknown4: u8,
    pub unknown5: u8,
    pub disable_unmorph: u8,
    pub disable_morph: u8,
    pub disable_controls: u8,
    pub disable_boost: u8,
    pub activate_visor_combat: u8,
    pub activate_visor_scan: u8,
    pub activate_visor_thermal: u8,
    pub activate_visor_xray: u8,
    pub unknown6: u8,
    pub face_object_on_unmorph: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PlayerHint<'r> {
    #[auto_struct(expect = 6)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,

    pub active: u8,
    pub data: PlayerHintStruct,
    pub priority: u32,
}

use crate::{impl_position, impl_rotation};
impl SclyPropertyData for PlayerHint<'_> {
    const OBJECT_TYPE: u8 = 0x3E;
    impl_position!();
    impl_rotation!();
}
