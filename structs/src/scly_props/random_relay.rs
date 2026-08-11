use crate::{impl_active, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::CStr;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct RandomRelay<'r> {
    #[auto_struct(expect = 5)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub send_set_size: u32,
    pub send_set_variance: u32,
    pub percent_size: u8,
    pub active: u8,
}

impl SclyPropertyData for RandomRelay<'_> {
    const OBJECT_TYPE: u8 = 0x14;

    impl_active!();
}
