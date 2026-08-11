use crate::{impl_active, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct Generator<'r> {
    #[auto_struct(expect = 8)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub spawn_count: u32,
    pub no_reuse_followers: u8,
    pub no_inherit_transform: u8,
    pub offset: GenericArray<f32, U3>,
    pub active: u8,
    pub min_scale: f32,
    pub max_scale: f32,
}

impl SclyPropertyData for Generator<'_> {
    const OBJECT_TYPE: u8 = 0xA;

    impl_active!();
}
