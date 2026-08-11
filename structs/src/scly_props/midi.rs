use crate::{impl_active, res_id::*, ResId, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::CStr;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct Midi<'r> {
    #[auto_struct(expect = 6)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub active: u8,
    pub song: ResId<CSNG>,
    pub fade_in: f32,
    pub fade_out: f32,
    pub volume: u32,
}

impl SclyPropertyData for Midi<'_> {
    const OBJECT_TYPE: u8 = 0x60;

    impl_active!();
}
