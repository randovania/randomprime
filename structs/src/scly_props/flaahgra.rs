use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    scly_props::structs::{
        ActorParameters, AnimationParameters, DamageInfo, DamageVulnerability, HealthInfo,
        PatternedInfo,
    },
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Flaahgra<'r> {
    #[auto_struct(expect = 23)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_params1: ActorParameters,
    pub dont_care: GenericArray<f32, U4>,
    pub damage_vulnerability: DamageVulnerability,
    pub dont_care0: u32,
    pub damage_info1: DamageInfo,
    pub dont_care1: u32,
    pub damage_info2: DamageInfo,
    pub dont_care2: u32,
    pub damage_info3: DamageInfo,
    pub actor_params2: ActorParameters,
    pub dont_care3: f32,
    pub dont_care4: f32,
    pub dont_care5: f32,
    pub anim_params: AnimationParameters,
    pub dont_care6: u32,
}

use crate::{impl_patterned_info, impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for Flaahgra<'_> {
    const OBJECT_TYPE: u8 = 0x4D;

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
        vec![
            self.patterned_info.damage_vulnerability.clone(),
            self.damage_vulnerability.clone(),
        ]
    }

    fn impl_set_vulnerabilities(&mut self, x: Vec<DamageVulnerability>) {
        self.patterned_info.damage_vulnerability = x[0].clone();
        self.damage_vulnerability = x[1].clone();
    }

    const SUPPORTS_HEALTH_INFOS: bool = true;

    fn impl_get_health_infos(&self) -> Vec<HealthInfo> {
        vec![self.patterned_info.health_info.clone()]
    }

    fn impl_set_health_infos(&mut self, x: Vec<HealthInfo>) {
        self.patterned_info.health_info = x[0].clone();
    }
}
