use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{impl_position, impl_rotation, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct AIJumpPoint<'r> {
    #[auto_struct(expect = 5)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub apex: f32,
}

impl SclyPropertyData for AIJumpPoint<'_> {
    const OBJECT_TYPE: u8 = 0x5B;

    impl_position!();
    impl_rotation!();
}
