use crate::{impl_active, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct ColorModulate<'r> {
    #[auto_struct(expect = 12)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub color_a: GenericArray<f32, U4>,
    pub color_b: GenericArray<f32, U4>,
    pub blend_mode: u32,
    pub time_a2b: f32,
    pub time_b2a: f32,
    pub do_reverse: u8,
    pub reset_target_when_done: u8,
    pub depth_compare: u8,
    pub depth_update: u8,
    pub depth_backwards: u8,
    pub active: u8,
}

impl SclyPropertyData for ColorModulate<'_> {
    const OBJECT_TYPE: u8 = 0x5E;

    impl_active!();
}
