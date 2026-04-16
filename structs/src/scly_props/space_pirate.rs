use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    impl_patterned_info, impl_position, impl_rotation, impl_scale, scly_props::structs::*,
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct SpacePirate<'r> {
    #[auto_struct(expect = 36)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_parameters: ActorParameters,

    pub dont_cares1: GenericArray<f32, U7>,
    pub dont_care: u8,

    pub wpsc1: u32,
    pub damage_info1: DamageInfo,
    pub sound1: u32,
    pub damage_info2: DamageInfo,
    pub unknown9: f32,
    pub wpsc2: u32,
    pub damage_info3: DamageInfo,
    pub dont_cares2: GenericArray<u32, U15>,
}

impl SclyPropertyData for SpacePirate<'_> {
    const OBJECT_TYPE: u8 = 0x24;

    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info!();

    const SUPPORTS_DAMAGE_INFOS: bool = true;

    fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
        vec![
            self.patterned_info.contact_damage,
            self.damage_info1,
            self.damage_info2,
            self.damage_info3,
        ]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.patterned_info.contact_damage = x[0];
        self.damage_info1 = x[1];
        self.damage_info2 = x[2];
        self.damage_info3 = x[3];
    }

    const SUPPORTS_VULNERABILITIES: bool = true;

    fn impl_get_vulnerabilities(&self) -> Vec<DamageVulnerability> {
        vec![self.patterned_info.damage_vulnerability.clone()]
    }

    fn impl_set_vulnerabilities(&mut self, x: Vec<DamageVulnerability>) {
        self.patterned_info.damage_vulnerability = x[0].clone();
    }

    const SUPPORTS_HEALTH_INFOS: bool = true;

    fn impl_get_health_infos(&self) -> Vec<HealthInfo> {
        vec![self.patterned_info.health_info.clone()]
    }

    fn impl_set_health_infos(&mut self, x: Vec<HealthInfo>) {
        self.patterned_info.health_info = x[0].clone();
    }
}
