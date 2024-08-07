use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    res_id::*,
    scly_props::structs::{
        ActorParameters, DamageInfo, DamageVulnerability, HealthInfo, PatternedInfo,
    },
    ResId, SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MetroidPrimeStage2<'r> {
    #[auto_struct(expect = 11)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub patterned_info: PatternedInfo,
    pub actor_parameters: ActorParameters,
    pub part1: ResId<PART>,
    pub damage_info: DamageInfo,
    pub elsc: ResId<ELSC>,
    pub unknown: u32,
    pub part2: ResId<PART>,
}

use crate::{impl_patterned_info, impl_position, impl_rotation, impl_scale};
impl<'r> SclyPropertyData for MetroidPrimeStage2<'r> {
    const OBJECT_TYPE: u8 = 0x83;
    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info!();

    const SUPPORTS_DAMAGE_INFOS: bool = true;

    fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
        vec![self.patterned_info.contact_damage, self.damage_info]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.patterned_info.contact_damage = x[0];
        self.damage_info = x[1];
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
