use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    res_id::*,
    scly_props::structs::{ActorParameters, AncsProp, DamageVulnerability, HealthInfo},
    ResId, SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Platform<'r> {
    #[auto_struct(expect = 19)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub extent: GenericArray<f32, U3>, // hitbox?
    pub scan_offset: GenericArray<f32, U3>,

    pub cmdl: ResId<CMDL>,
    pub ancs: AncsProp,
    pub actor_params: ActorParameters,

    pub speed: f32,
    pub active: u8,

    pub dcln: ResId<DCLN>,

    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,

    // Consider a platform on a waypoint path with the player obstructing that path:
    // - if this is set to 0, then the platform will halt
    // - if this is set to 1, then the platform will slide around the player and fall off of the waypoint path (and it will wander aimlessly)
    pub detect_collision: u8,

    pub unknown4: f32,
    pub unknown5: u8,
    pub unknown6: u32,
    pub unknown7: u32,
}

use crate::{impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for Platform<'_> {
    const OBJECT_TYPE: u8 = 0x8;
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
