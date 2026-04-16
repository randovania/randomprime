use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraFilterKeyframe<'r> {
    #[auto_struct(expect = 10)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub active: u8,
    pub filter_type: u32,
    pub filter_shape: u32,
    pub filter_index: u32,
    pub filter_group: u32,
    pub color: GenericArray<f32, U4>, // RGBA
    pub fade_in_time: f32,
    pub fade_out_time: f32,
    pub overlay_texture: u32,
}

impl SclyPropertyData for CameraFilterKeyframe<'_> {
    const OBJECT_TYPE: u8 = 0x18;
}
