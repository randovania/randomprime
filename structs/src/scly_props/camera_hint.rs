use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::U3, CStr};

use crate::{
    scly_props::structs::{BoolFloat, BoolVec3, CameraHintParameters},
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraHint<'r> {
    #[auto_struct(expect = 23)]
    prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub priority: u32,
    pub behavior: u32,
    pub camera_hint_params: CameraHintParameters,
    pub min_dist: BoolFloat,
    pub max_dist: BoolFloat,
    pub backwards_dist: BoolFloat,
    pub look_at_offset: BoolVec3,
    pub chase_look_at_offset: BoolVec3,
    pub ball_to_cam: GenericArray<f32, U3>,
    pub fov: BoolFloat,
    pub attitude_range: BoolFloat,
    pub azimuth_range: BoolFloat,
    pub angle_per_second: BoolFloat,
    pub clamp_vel_range: f32,
    pub clamp_rot_range: f32,
    pub elevation: BoolFloat,
    pub interpolate_time: f32,
    pub clamp_vel_time: f32,
    pub control_interp_dur: f32,
}

use crate::{impl_position, impl_rotation};
impl SclyPropertyData for CameraHint<'_> {
    const OBJECT_TYPE: u8 = 0x10;
    impl_position!();
    impl_rotation!();
}
