use auto_struct_macros::auto_struct;
use reader_writer::CStr;

use crate::{impl_active, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct CameraBlurKeyframe<'r> {
    #[auto_struct(expect = 7)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub active: u8,
    pub blur_type: u32,
    pub amount: f32,
    pub unknown: u32,
    pub time_in: f32,
    pub time_out: f32,
}

impl SclyPropertyData for CameraBlurKeyframe<'_> {
    const OBJECT_TYPE: u8 = 0x19;

    impl_active!();
}
