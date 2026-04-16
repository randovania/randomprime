use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct DistanceFog<'r> {
    #[auto_struct(expect = 8)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub mode: u32,
    pub color: GenericArray<f32, U4>,
    pub range: GenericArray<f32, U2>,
    pub color_delta: f32,
    pub range_delta: GenericArray<f32, U2>,
    pub explicit: u8,
    pub active: u8,
}

impl SclyPropertyData for DistanceFog<'_> {
    const OBJECT_TYPE: u8 = 0x35;
}
