use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct GrappleParameters {
    #[auto_struct(expect = 12)]
    prop_count: u32,

    pub grapple_length: f32,
    pub grapple_attach_length: f32,
    pub grapple_spring_constant: f32,
    pub grapple_spring_length: f32,
    pub grapple_spring_tardis: f32,
    pub swing_force: f32,
    pub swing_max_force: f32,
    pub swing_arc_angle: f32,
    pub swing_turn_angle: f32,
    pub swing_camera_pitch: f32,
    pub swing_camera_max_pitch: f32,

    pub constrain_to_axis: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct GrapplePoint<'r> {
    #[auto_struct(expect = 5)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,

    pub active: u8,

    pub grapple_parameters: GrappleParameters,
}

use crate::{impl_active, impl_position, impl_rotation};
impl SclyPropertyData for GrapplePoint<'_> {
    const OBJECT_TYPE: u8 = 0x30;
    impl_active!();
    impl_position!();
    impl_rotation!();
}
