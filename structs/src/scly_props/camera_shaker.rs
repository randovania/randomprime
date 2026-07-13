use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct CameraShaker<'r> {
    #[auto_struct(expect = 9)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub dont_care: GenericArray<u8, U29>,
}

impl SclyPropertyData for CameraShaker<'_> {
    const OBJECT_TYPE: u8 = 0x1C;
}
