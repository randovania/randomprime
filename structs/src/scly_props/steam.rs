use crate::{
    impl_active, impl_position, res_id::*, scly_props::structs::DamageInfo, ResId, SclyPropertyData,
};
use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct Steam<'r> {
    #[auto_struct(expect = 11)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub extent: GenericArray<f32, U3>,
    pub damage_info: DamageInfo,
    pub oriented_force: GenericArray<f32, U3>,
    pub trigger_flags: u32,
    pub active: u8,
    pub texture: ResId<TXTR>,
    pub unknown0: f32,
    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,
    pub unknown4: u8,
}

impl SclyPropertyData for Steam<'_> {
    const OBJECT_TYPE: u8 = 0x46;

    impl_active!();
    impl_position!();
}
