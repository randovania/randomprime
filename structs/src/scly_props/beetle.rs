use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{scly_structs::*, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Beetle<'r> {
    #[auto_struct(expect = 16)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub flavor: f32,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,
    pub touch_damage: DamageInfo,
    pub tail_aim_reference: GenericArray<f32, U3>,
    pub unused: f32,
    pub damage_vulnerability1: DamageVulnerability,
    pub damage_vulnerability2: DamageVulnerability,
    pub tail_cmdl: f32,
    pub entrance_type: f32,
    pub initial_attack_delay: f32,
    pub retreat_time: f32,
}

use crate::{impl_patterned_info, impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for Beetle<'_> {
    const OBJECT_TYPE: u8 = 0x16;

    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info!();

    const SUPPORTS_DAMAGE_INFOS: bool = true;

    fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
        vec![self.patterned_info.contact_damage, self.touch_damage]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.patterned_info.contact_damage = x[0];
        self.touch_damage = x[1];
    }

    const SUPPORTS_VULNERABILITIES: bool = true;

    fn impl_get_vulnerabilities(&self) -> Vec<DamageVulnerability> {
        vec![
            self.patterned_info.damage_vulnerability.clone(),
            self.damage_vulnerability1.clone(),
            self.damage_vulnerability2.clone(),
        ]
    }

    fn impl_set_vulnerabilities(&mut self, x: Vec<DamageVulnerability>) {
        self.patterned_info.damage_vulnerability = x[0].clone();
        self.damage_vulnerability1 = x[1].clone();
        self.damage_vulnerability2 = x[2].clone();
    }

    const SUPPORTS_HEALTH_INFOS: bool = true;

    fn impl_get_health_infos(&self) -> Vec<HealthInfo> {
        vec![self.patterned_info.health_info.clone()]
    }

    fn impl_set_health_infos(&mut self, x: Vec<HealthInfo>) {
        self.patterned_info.health_info = x[0].clone();
    }
}
