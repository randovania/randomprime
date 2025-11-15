use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct VisorFlare<'r> {
    #[auto_struct(expect = 14)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub dont_care: GenericArray<u8, U194>,
}

impl SclyPropertyData for VisorFlare<'_> {
    const OBJECT_TYPE: u8 = 0x51;
}
