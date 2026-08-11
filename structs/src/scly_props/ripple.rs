use crate::{impl_active, impl_position, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct Ripple<'r> {
    #[auto_struct(expect = 4)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub active: u8,
    pub magnitude: f32,
}

impl SclyPropertyData for Ripple<'_> {
    const OBJECT_TYPE: u8 = 0x47;

    impl_active!();
    impl_position!();
}
