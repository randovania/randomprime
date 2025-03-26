use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PickupGenerator<'r> {
    #[auto_struct(expect = 4)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub offset: GenericArray<f32, U3>,
    pub active: u8,
    pub frequency: f32,
}

impl SclyPropertyData for PickupGenerator<'_> {
    const OBJECT_TYPE: u8 = 0x40;
}
