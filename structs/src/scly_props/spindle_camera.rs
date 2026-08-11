use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr, LazyArray};

use crate::{impl_active, impl_position, impl_rotation, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct SpindleProperty<'r> {
    pub input: u32,

    #[auto_struct(derive = parameter_flags.len() as u32)]
    parameter_flag_count: u32,
    #[auto_struct(init = (parameter_flag_count as usize, ()))]
    pub parameter_flags: LazyArray<'r, u8>,

    pub low_out: f32,
    pub high_out: f32,
    pub low_in: f32,
    pub high_in: f32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct SpindleCamera<'r> {
    #[auto_struct(expect = 24)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,

    #[auto_struct(derive = parameter_flags.len() as u32)]
    parameter_flag_count: u32,
    #[auto_struct(init = (parameter_flag_count as usize, ()))]
    pub parameter_flags: LazyArray<'r, u8>,

    pub hint_to_cam_dist_min: f32,
    pub hint_to_cam_dist_max: f32,
    pub hint_to_cam_v_off_min: f32,
    pub hint_to_cam_v_off_max: f32,

    #[auto_struct(init = (15, ()))]
    pub segments: LazyArray<'r, SpindleProperty<'r>>,
}

impl SclyPropertyData for SpindleCamera<'_> {
    const OBJECT_TYPE: u8 = 0x71;

    impl_active!();
    impl_position!();
    impl_rotation!();
}
