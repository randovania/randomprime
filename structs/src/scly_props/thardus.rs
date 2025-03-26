use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    scly_props::structs::{
        ActorParameters, DamageInfo, DamageVulnerability, HealthInfo, PatternedInfo,
    },
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Thardus<'r> {
    #[auto_struct(expect = 44)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub patterned_info: PatternedInfo,
    pub actor_parameters: ActorParameters,
    pub unknown1: u8,
    pub unknown2: u8,
    pub asset_ids: GenericArray<u32, U24>,
    pub values: GenericArray<f32, U6>,
    pub asset_ids2: GenericArray<u32, U6>,
}

use crate::{impl_patterned_info_with_auxillary, impl_position, impl_rotation, impl_scale};
impl SclyPropertyData for Thardus<'_> {
    const OBJECT_TYPE: u8 = 0x58;
    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info_with_auxillary!();
}
