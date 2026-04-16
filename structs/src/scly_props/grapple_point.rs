use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct GrappleParams {
    #[auto_struct(expect = 12)]
    prop_count: u32,

    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,
    pub unknown4: f32,
    pub unknown5: f32,
    pub unknown6: f32,
    pub unknown7: f32,
    pub unknown8: f32,
    pub unknown9: f32,
    pub unknown10: f32,
    pub unknown11: f32,

    pub disable_turning: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct GrapplePoint<'r> {
    #[auto_struct(expect = 5)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,

    pub active: u8,

    pub grapple_params: GrappleParams,
}

use crate::{impl_position, impl_rotation};
impl SclyPropertyData for GrapplePoint<'_> {
    const OBJECT_TYPE: u8 = 0x30;
    impl_position!();
    impl_rotation!();
}
