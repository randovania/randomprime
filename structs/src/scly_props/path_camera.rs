use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::U3, CStr};

use crate::{scly_props::structs::PathCameraFlags, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PathCamera<'r> {
    #[auto_struct(expect = 11)]
    prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub flags: PathCameraFlags,
    pub length_extend: f32,
    pub filter_mag: f32,
    pub filter_proportion: f32,
    pub initial_spline_position: u32,
    pub min_ease_dist: f32,
    pub max_ease_dist: f32,
}

use crate::{impl_position, impl_rotation};
impl SclyPropertyData for PathCamera<'_> {
    const OBJECT_TYPE: u8 = 0x2F;
    impl_position!();
    impl_rotation!();
}
