use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    impl_patterned_info_with_auxillary, impl_position, impl_rotation, impl_scale,
    scly_props::structs::*, SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Parasite<'r> {
    #[auto_struct(expect = 25)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub unknown1: u32,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_parameters: ActorParameters,

    pub dont_cares: GenericArray<f32, U17>,
    pub dont_care: u8,
}

impl<'r> SclyPropertyData for Parasite<'r> {
    const OBJECT_TYPE: u8 = 0x3D;

    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info_with_auxillary!();
}
