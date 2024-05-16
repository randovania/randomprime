use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::U3, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct CameraHintParameters {
    #[auto_struct(expect = 15)]
    prop_count: u32,
    pub calculate_cam_pos: u8,
    pub chase_allowed: u8,
    pub boost_allowed: u8,
    pub obscure_avoidance: u8,
    pub volume_collider: u8,
    pub apply_immediately: u8,
    pub look_at_ball: u8,
    pub hint_distance_selection: u8,
    pub hint_distance_self_pos: u8,
    pub control_interpolation: u8,
    pub sinusoidal_interpolation: u8,
    pub sinusoidal_interpolation_hintless: u8,
    pub clamp_velocity: u8,
    pub skip_cinematic: u8,
    pub no_elevation_interp: u8,
    pub direct_elevation: u8,
    pub override_look_dir: u8,
    pub no_elevation_vel_clamp: u8,
    pub calculate_transform_from_prev_cam: u8,
    pub no_spline: u8,
    pub unknown21: u8,
    pub unknown22: u8,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct BoolFloat {
    pub active: u8,
    pub value: f32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct BoolVec3 {
    pub active: u8,
    pub value: GenericArray<f32, U3>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraHint<'r> {
    #[auto_struct(expect = 9)]
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

// cannot implement position/rotation until the size of this struct is corrected
impl<'r> SclyPropertyData for CameraHint<'r> {
    const OBJECT_TYPE: u8 = 0x10;
}
