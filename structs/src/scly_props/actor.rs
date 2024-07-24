use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    res_id::*,
    scly_props::structs::{ActorParameters, AncsProp, DamageVulnerability, HealthInfo},
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Actor<'r> {
    #[auto_struct(expect = 24)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub collision_box: GenericArray<f32, U3>,
    pub collision_offset: GenericArray<f32, U3>,

    pub mass: f32,
    pub momentum: f32,

    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,

    pub cmdl: ResId<CMDL>,
    pub ancs: AncsProp,
    pub actor_params: ActorParameters,

    pub is_loop: u8,
    pub immovable   : u8,
    pub is_solid: u8,
    pub is_camera_through: u8,
    pub active: u8,
    pub render_texture_set: u32,
    pub xray_alpha: f32,
    pub thermal_visible_through_geometry: u8,
    pub draws_shadow: u8,
    pub scale_animation: u8,
    pub material_flag_54: u8,
}

use crate::{impl_position, impl_rotation, impl_scale};
impl<'r> SclyPropertyData for Actor<'r> {
    const OBJECT_TYPE: u8 = 0x0;
    impl_position!();
    impl_rotation!();
    impl_scale!();

    const SUPPORTS_VULNERABILITIES: bool = true;

    fn impl_get_vulnerabilities(&self) -> Vec<DamageVulnerability> {
        vec![self.damage_vulnerability.clone()]
    }

    fn impl_set_vulnerabilities(&mut self, x: Vec<DamageVulnerability>) {
        self.damage_vulnerability = x[0].clone();
    }

    const SUPPORTS_HEALTH_INFOS: bool = true;

    fn impl_get_health_infos(&self) -> Vec<HealthInfo> {
        vec![self.health_info.clone()]
    }

    fn impl_set_health_infos(&mut self, x: Vec<HealthInfo>) {
        self.health_info = x[0].clone();
    }
}
