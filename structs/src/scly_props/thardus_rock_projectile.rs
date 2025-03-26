use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    impl_patterned_info_with_auxillary, impl_position, impl_rotation, impl_scale,
    scly_props::structs::*, SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ThardusRockProjectile<'r> {
    #[auto_struct(expect = 11)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_parameters: ActorParameters,

    pub unknown1: u8,
    pub unknown2: u8,
    pub unknown3: f32,
    pub cmdl: u32,
    pub afsm: u32,
}

impl SclyPropertyData for ThardusRockProjectile<'_> {
    const OBJECT_TYPE: u8 = 0x5F;

    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info_with_auxillary!();
}
