use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct Generator<'r> {
    #[auto_struct(expect = 8)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub dont_care: GenericArray<u8, U27>,
}

impl SclyPropertyData for Generator<'_> {
    const OBJECT_TYPE: u8 = 0x0A;
}
