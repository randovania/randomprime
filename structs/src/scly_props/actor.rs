use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    res_id::*,
    scly_props::structs::{ActorParameters, AncsProp, DamageVulnerability, HealthInfo},
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Actor<'r> {
    #[auto_struct(expect = 24)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub hitbox: GenericArray<f32, U3>,
    pub scan_offset: GenericArray<f32, U3>,

    pub unknown1: f32,
    pub unknown2: f32,

    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,

    pub cmdl: ResId<CMDL>,
    pub ancs: AncsProp,
    pub actor_params: ActorParameters,

    pub looping: u8,
    pub snow: u8,
    pub solid: u8,
    pub camera_passthrough: u8,
    pub active: u8,
    pub unknown8: u32,
    pub unknown9: f32,
    pub unknown10: u8,
    pub unknown11: u8,
    pub unknown12: u8,
    pub unknown13: u8,
}

use crate::{impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for Actor<'_> {
    const OBJECT_TYPE: u8 = 0x0;
    impl_position!();
    impl_rotation!();
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
