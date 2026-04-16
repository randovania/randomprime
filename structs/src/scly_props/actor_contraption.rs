use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    scly_props::structs::{
        ActorParameters, AnimationParameters, DamageInfo, DamageVulnerability, HealthInfo,
    },
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ActorContraption<'r> {
    #[auto_struct(expect = 15)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub dont_care0: GenericArray<f32, U8>,
    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,
    pub animation_params: AnimationParameters,
    pub actor_params: ActorParameters,
    pub dont_care1: u32,
    pub damage_info: DamageInfo,
    pub dont_care2: u8,
}

use crate::{impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for ActorContraption<'_> {
    const OBJECT_TYPE: u8 = 0x6E;

    impl_position!();
    impl_rotation!();
    impl_scale!();

    const SUPPORTS_DAMAGE_INFOS: bool = true;

    fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
        vec![self.damage_info]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.damage_info = x[0];
    }

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
