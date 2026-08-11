use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{impl_active, impl_position, res_id::*, ResId, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct FlareDef {
    #[auto_struct(expect = 4)]
    pub prop_count: u32,

    pub texture: ResId<TXTR>,
    pub position: f32,
    pub scale: f32,
    pub color: GenericArray<f32, U4>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct VisorFlare<'r> {
    #[auto_struct(expect = 14)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub active: u8,
    pub blend_mode: u32,
    pub unknown0: u8,
    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,
    pub unknown4: u32,

    // Always five slots on disk; a slot with an invalid texture id is skipped at load
    pub flares: GenericArray<FlareDef, U5>,
}

impl SclyPropertyData for VisorFlare<'_> {
    const OBJECT_TYPE: u8 = 0x51;

    impl_active!();
    impl_position!();
}
