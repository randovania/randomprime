use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{scly_structs::*, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ElitePirate<'r> {
    #[auto_struct(expect = 42)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,

    pub unknowns: GenericArray<f32, U8>,
    pub part1: u32,
    pub sound_id1: u32,

    pub actor_params2: ActorParameters,
    pub animation_params: AnimationParameters,

    pub part2: u32,
    pub sound_id2: u32,

    pub cmdl: u32,
    pub damage_info1: DamageInfo,

    pub unknown9: f32,
    pub part3: u32,
    pub part4: u32,
    pub part5: u32,
    pub part6: u32,
    pub unknown10: f32,
    pub unknown11: f32,
    pub unknown12: f32,
    pub unknown13: f32,
    pub unknown14: f32,
    pub unknown15: f32,
    pub unknown16: u32,
    pub sound_id3: u32,
    pub sound_id4: u32,
    pub part7: u32,
    pub damage_info2: DamageInfo,
    pub elsc: u32,
    pub sound_id5: u32,
    pub unknown17: u8,
    pub unknown18: u8,
}

use crate::{impl_patterned_info, impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for ElitePirate<'_> {
    const OBJECT_TYPE: u8 = 0x26;

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
