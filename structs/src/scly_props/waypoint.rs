use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Waypoint<'r> {
    #[auto_struct(expect = 13)]
    prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub speed: f32,
    pub pause: f32,
    pub pattern_translate: u32,
    pub pattern_orient: u32,
    pub pattern_fit: u32,
    pub behaviour: u32,
    pub behaviour_orient: u32,
    pub behaviour_modifiers: u32,
    pub animation: u32,
}

use crate::{impl_position, impl_rotation};
impl SclyPropertyData for Waypoint<'_> {
    const OBJECT_TYPE: u8 = 0x02;
    impl_position!();
    impl_rotation!();
}
