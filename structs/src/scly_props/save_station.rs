use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct SaveStation<'r> {
    #[auto_struct(expect = 1)]
    pub prop_count: u32,

    pub name: CStr<'r>,
}

impl SclyPropertyData for SaveStation<'_> {
    const OBJECT_TYPE: u8 = 0x59;
}
