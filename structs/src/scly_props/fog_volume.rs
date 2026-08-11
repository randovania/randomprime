use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{impl_active, impl_position, impl_scale, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct FogVolume<'r> {
    #[auto_struct(expect = 7)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub flicker_speed: f32,
    pub unknown: f32,
    pub color: GenericArray<f32, U4>,
    pub active: u8,
}

impl SclyPropertyData for FogVolume<'_> {
    const OBJECT_TYPE: u8 = 0x65;

    impl_position!();
    impl_scale!();
    impl_active!();
}
