use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{impl_position, impl_rotation, impl_scale, scly_props::structs::*, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PhazonPool<'r> {
    #[auto_struct(expect = 18)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub dont_care1: u8,
    pub dont_cares1: GenericArray<u32, U5>,
    pub damage_info: DamageInfo,
    pub dont_cares2: GenericArray<u32, U7>,
    pub dont_care2: u8,
    pub dont_care3: u32,
}

impl SclyPropertyData for PhazonPool<'_> {
    const OBJECT_TYPE: u8 = 0x87;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
