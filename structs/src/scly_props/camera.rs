use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Camera<'r> {
    #[auto_struct(expect = 15)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub shot_duration: f32,

    // Value<bool> lookAtPlayer;
    // Value<bool> outOfPlayerEye;
    // Value<bool> intoPlayerEye;
    // Value<bool> drawPlayer;
    // Value<bool> disableInput;
    // Value<bool> unknown;
    // Value<bool> finishCineSkip;
    // Value<float> fov;
    // Value<bool> checkFailsafe;
    // Value<bool> disableOutOfInto;
    pub unknowns: GenericArray<u8, U7>,
    pub unknown1: f32,
    pub unknown2: u8,
    pub unknown3: u8,
}

use crate::{impl_position, impl_rotation};
impl<'r> SclyPropertyData for Camera<'r> {
    const OBJECT_TYPE: u8 = 0x0C;

    impl_position!();
    impl_rotation!();
}
