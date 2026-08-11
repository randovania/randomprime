use crate::{impl_active, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::CStr;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct CameraShaker<'r> {
    #[auto_struct(expect = 9)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub horizontal_shake: f32,
    pub unknown0: f32,
    pub unknown1: f32,
    pub unknown2: f32,
    pub vertical_shake: f32,
    pub unknown3: f32,
    pub shake_duration: f32,
    pub active: u8,
}

impl SclyPropertyData for CameraShaker<'_> {
    const OBJECT_TYPE: u8 = 0x1C;

    impl_active!();
}
