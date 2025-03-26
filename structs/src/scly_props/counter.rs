use auto_struct_macros::auto_struct;
use reader_writer::CStr;

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Counter<'r> {
    #[auto_struct(expect = 5)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub start_value: u32,
    pub max_value: u32,
    pub auto_reset: u8,
    pub active: u8,
}

impl SclyPropertyData for Counter<'_> {
    const OBJECT_TYPE: u8 = 0x06;
}
