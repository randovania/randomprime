use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{impl_position, impl_rotation, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraWaypoint<'r> {
    #[auto_struct(expect = 6)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,

    pub active: u8,
    pub fov: f32,
    pub unknown: u32,
}

impl SclyPropertyData for CameraWaypoint<'_> {
    const OBJECT_TYPE: u8 = 0xD;

    impl_position!();
    impl_rotation!();
}
