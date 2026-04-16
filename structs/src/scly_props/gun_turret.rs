use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{impl_position, impl_rotation, impl_scale, scly_props::structs::*, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct GunTurret<'r> {
    #[auto_struct(expect = 48)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub unknown1: u32,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub collision_extent: GenericArray<f32, U3>, // TODO: scale
    pub collision_offset: GenericArray<f32, U3>,

    pub animation_params: AnimationParameters,
    pub actor_params: ActorParameters,

    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,

    pub into_deactivate_delay: f32,
    pub reload_time: f32,
    pub reload_time_variance: f32,

    pub dont_care: GenericArray<u8, U146>,
}

impl SclyPropertyData for GunTurret<'_> {
    const OBJECT_TYPE: u8 = 0x64;

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
        self.health_info = x[0].clone()
    }
}
