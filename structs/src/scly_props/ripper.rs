use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    impl_patterned_info_with_auxillary, impl_position, impl_rotation, impl_scale,
    scly_props::structs::*, SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Ripper<'r> {
    #[auto_struct(expect = 8)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub unknown1: u32,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_parameters: ActorParameters,
    pub grapple_parameters: GrappleParameters,
}

impl SclyPropertyData for Ripper<'_> {
    const OBJECT_TYPE: u8 = 0x3F;

    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info_with_auxillary!();
}
