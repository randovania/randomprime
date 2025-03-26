use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{impl_position, impl_rotation, impl_scale, scly_props::structs::*, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct DebrisExtended<'r> {
    #[auto_struct(expect = 39)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub dont_cares1: GenericArray<f32, U27>,
    pub actor_params: ActorParameters,
    pub part: u32,
    pub dont_care1: GenericArray<f32, U3>,
    pub dont_care2: u8,
    pub dont_care3: u8,
    pub dont_cares2: GenericArray<f32, U5>,
    pub dont_care4: u8,
    pub dont_care5: u8,
    pub dont_cares3: GenericArray<f32, U6>,
    pub dont_cares4: GenericArray<u8, U4>,
}

impl SclyPropertyData for DebrisExtended<'_> {
    const OBJECT_TYPE: u8 = 0x45;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
