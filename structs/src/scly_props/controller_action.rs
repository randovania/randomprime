use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ControllerAction<'r>
{
    #[auto_struct(expect = 4)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub active: u8,
    pub action_: u32,
    pub one_shot: u8,
}

impl<'r> SclyPropertyData for ControllerAction<'r>
{
    const OBJECT_TYPE: u8 = 0x55;
}
