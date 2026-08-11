use crate::{impl_active, impl_position, impl_rotation, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct MazeNode<'r> {
    #[auto_struct(expect = 10)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub column: u32,
    pub row: u32,
    pub side: u32,
    pub actor_position: GenericArray<f32, U3>,
    pub trigger_position: GenericArray<f32, U3>,
    pub effect_position: GenericArray<f32, U3>,
}

impl SclyPropertyData for MazeNode<'_> {
    const OBJECT_TYPE: u8 = 0x85;

    impl_active!();
    impl_position!();
    impl_rotation!();
}
