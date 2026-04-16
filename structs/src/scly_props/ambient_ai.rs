use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{impl_position, impl_rotation, impl_scale, scly_props::structs::*, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct AmbientAI<'r> {
    #[auto_struct(expect = 16)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub collision_extent: GenericArray<f32, U3>,
    pub collision_offset: GenericArray<f32, U3>,
    pub mass: f32,
    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,
    pub animation_params: AnimationParameters,
    pub alert_range: f32,
    pub impact_range: f32,
    pub alert_anim: u32,
    pub impact_anim: u32,
    pub active: u8,

    pub dont_care: GenericArray<u8, U125>,
}

impl SclyPropertyData for AmbientAI<'_> {
    const OBJECT_TYPE: u8 = 0x75;

    impl_position!();
    impl_rotation!();
    impl_scale!(); // TODO: scale should also affect collision extent and mass

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
