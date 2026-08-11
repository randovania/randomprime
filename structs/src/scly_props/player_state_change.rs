use crate::{impl_active, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::CStr;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerStateChange<'r> {
    #[auto_struct(expect = 7)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub active: u8,
    pub item_type: u32,
    pub item_count: u32,
    pub item_capacity: u32,
    pub control: u32,
    pub control_command_option: u32,
}

impl SclyPropertyData for PlayerStateChange<'_> {
    const OBJECT_TYPE: u8 = 0x57;

    impl_active!();
}
