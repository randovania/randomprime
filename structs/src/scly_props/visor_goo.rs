use crate::{impl_position, res_id::*, ResId, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct VisorGoo<'r> {
    #[auto_struct(expect = 11)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub particle: ResId<PART>,
    pub electric: ResId<ELSC>,
    pub min_distance: f32,
    pub max_distance: f32,
    pub near_probability: f32,
    pub far_probability: f32,
    pub color: GenericArray<f32, U4>,
    pub sfx: u32,
    pub force_show: u8,
}

impl SclyPropertyData for VisorGoo<'_> {
    const OBJECT_TYPE: u8 = 0x53;

    impl_position!();
}
