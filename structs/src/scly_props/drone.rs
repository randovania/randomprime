use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{scly_structs::*, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Drone<'r> {
    #[auto_struct(expect = 45)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub unknown1: u32,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub unknown2: f32,

    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,
    pub damage_info1: DamageInfo,
    pub unknown3: f32,
    pub damage_info2: DamageInfo,

    pub dont_care: GenericArray<u8, U273>,
}

use crate::{impl_patterned_info, impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for Drone<'_> {
    const OBJECT_TYPE: u8 = 0x43;

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
        ]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.patterned_info.contact_damage = x[0];
        self.damage_info1 = x[1];
        self.damage_info2 = x[2];
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
