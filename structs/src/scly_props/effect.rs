use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{res_id::*, scly_props::structs::LightParameters, ResId, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Effect<'r> {
    #[auto_struct(expect = 24)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub part: ResId<PART>,
    pub elsc: ResId<ELSC>,

    pub hot_in_thermal: u8,
    pub no_timer_unless_area_occluded: u8,
    pub rebuild_systems_on_active: u8,
    pub active: u8,
    pub use_rate_inverse_cam_dist: u8,
    pub rate_inverse_cam_dist: f32,
    pub rate_inverse_cam_dist_rate: f32,
    pub duration: f32,
    pub dureation_reset_while_visible: f32,
    pub use_rate_cam_dist_range: u8,
    pub rate_cam_dist_range_min: f32,
    pub rate_cam_dist_range_max: f32,
    pub rate_cam_dist_range_far_rate: f32,
    pub combat_visor_visible: u8,
    pub thermal_visor_visible: u8,
    pub xray_visor_visible: u8,
    pub die_when_systems_done: u8,

    pub light_params: LightParameters,
}

use crate::{impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for Effect<'_> {
    const OBJECT_TYPE: u8 = 0x7;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
