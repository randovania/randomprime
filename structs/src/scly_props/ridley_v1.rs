use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    res_id::*,
    scly_props::structs::{
        ActorParameters, DamageInfo, PatternedInfo, RidleyStruct1, RidleyStruct2,
    },
    scly_structs::*,
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct RidleyV1<'r> {
    #[auto_struct(expect = 48)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,

    pub models: GenericArray<ResId<CMDL>, U12>,
    pub particle: ResId<PART>,

    pub unknown0: f32,
    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,

    pub wpsc0: u32, // missing ResId<WPSC>
    pub damage_info1: DamageInfo,
    pub ridley_struct1_1: RidleyStruct1,
    pub sound0: u32,
    pub wpsc1: u32, // missing ResId<WPSC>
    pub damage_info2: DamageInfo,
    pub ridley_struct2_1: RidleyStruct2,
    pub wpsc2: u32, // missing ResId<WPSC>
    pub damage_info3: DamageInfo,
    pub ridley_struct2_2: RidleyStruct2,
    pub sound1: u32, // missing ResId<WPSC>
    pub damage_info4: DamageInfo,
    pub ridley_struct2_3: RidleyStruct2,
    pub unknown4: f32,
    pub unknown5: f32,
    pub damage_info5: DamageInfo,
    pub unknown6: f32,
    pub damage_info6: DamageInfo,
    pub unknown7: f32,
    pub damage_info7: DamageInfo,
    pub unknown8: f32,
    pub elsc: u32,
    pub unknown9: f32,
    pub sound2: u32,
    pub damage_info8: DamageInfo,
}

use crate::{impl_patterned_info, impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for RidleyV1<'_> {
    const OBJECT_TYPE: u8 = 0x7B;
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
            self.damage_info4,
            self.damage_info5,
            self.damage_info6,
            self.damage_info7,
            self.damage_info8,
        ]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.patterned_info.contact_damage = x[0];
        self.damage_info1 = x[1];
        self.damage_info2 = x[2];
        self.damage_info3 = x[3];
        self.damage_info4 = x[4];
        self.damage_info5 = x[5];
        self.damage_info6 = x[6];
        self.damage_info7 = x[7];
        self.damage_info8 = x[8];
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
