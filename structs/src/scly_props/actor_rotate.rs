use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{impl_rotation, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct ActorRotate<'r> {
    #[auto_struct(expect = 6)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub rotation: GenericArray<f32, U3>,
    pub time_scale: f32,
    pub update_actors: u8,
    pub update_on_creation: u8,
    pub update_active: u8,
}

impl SclyPropertyData for ActorRotate<'_> {
    const OBJECT_TYPE: u8 = 0x39;

    const SUPPORTS_ACTIVE: bool = true;

    fn impl_get_active(&self) -> u8 {
        self.update_active
    }

    fn impl_set_active(&mut self, x: u8) {
        self.update_active = x;
    }

    impl_rotation!();
}
