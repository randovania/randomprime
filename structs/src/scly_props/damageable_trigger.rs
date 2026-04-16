use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    res_id::*,
    scly_props::structs::{DamageVulnerability, HealthInfo, VisorParameters},
    ResId, SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct DamageableTrigger<'r> {
    #[auto_struct(expect = 12)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,

    pub unknown0: u32,

    pub pattern_txtr0: ResId<TXTR>,
    pub pattern_txtr1: ResId<TXTR>,
    pub color_txtr: ResId<TXTR>,

    pub lock_on: u8,
    pub active: u8,

    pub visor_params: VisorParameters,
}

use crate::{impl_position, impl_scale};
impl SclyPropertyData for DamageableTrigger<'_> {
    const OBJECT_TYPE: u8 = 0x1A;

    impl_position!();
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
        self.health_info = x[0].clone();
    }
}
