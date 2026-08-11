use crate::{impl_active, impl_position, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowProjector<'r> {
    #[auto_struct(expect = 10)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub active: u8,
    pub scale: f32,
    pub offset: GenericArray<f32, U3>,
    pub z_offset_adjust: f32,
    pub opacity: f32,
    pub opacity_recip: f32,
    pub persistent: u8,
    pub texture_size: u32,
}

impl SclyPropertyData for ShadowProjector<'_> {
    const OBJECT_TYPE: u8 = 0x8A;

    impl_active!();
    impl_position!();
}
