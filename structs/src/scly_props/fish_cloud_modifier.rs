use crate::{impl_active, impl_position, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct FishCloudModifier<'r> {
    #[auto_struct(expect = 7)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub active: u8,
    pub is_repulsor: u8,
    pub swirl: u8,
    pub radius: f32,
    pub priority: f32,
}

impl SclyPropertyData for FishCloudModifier<'_> {
    const OBJECT_TYPE: u8 = 0x50;

    impl_active!();
    impl_position!();
}
