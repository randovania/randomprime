use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct SpiderBallWaypoint<'r> {
    #[auto_struct(expect = 5)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub unknown2: u32,
}

use crate::{impl_position, impl_rotation};
impl SclyPropertyData for SpiderBallWaypoint<'_> {
    const OBJECT_TYPE: u8 = 0x2C;
    impl_position!();
    impl_rotation!();
}
