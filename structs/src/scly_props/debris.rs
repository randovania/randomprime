use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{impl_position, impl_rotation, impl_scale, scly_props::structs::*, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Debris<'r> {
    #[auto_struct(expect = 18)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub dont_cares1: GenericArray<f32, U12>,
    pub dont_care1: u8,
    pub cmdl: u32,
    pub actor_params: ActorParameters,
    pub dont_cares2: GenericArray<u32, U4>,
    pub dont_care2: u8,
    pub dont_care3: u8,
}

impl SclyPropertyData for Debris<'_> {
    const OBJECT_TYPE: u8 = 0x1B;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
