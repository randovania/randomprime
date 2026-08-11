use crate::{impl_active, impl_position, impl_rotation, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct TargetingPoint<'r> {
    #[auto_struct(expect = 4)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
}

impl SclyPropertyData for TargetingPoint<'_> {
    const OBJECT_TYPE: u8 = 0x49;

    impl_active!();
    impl_position!();
    impl_rotation!();
}
