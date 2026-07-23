use auto_struct_macros::auto_struct;
use reader_writer::CStr;

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct ActorKeyFrame<'r> {
    #[auto_struct(expect = 7)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub animation_index: u32,
    pub loop_: u8,
    pub loop_duration: f32,
    pub active: u8,
    pub fade_out: u32,
    pub playback_rate: f32,
}

use crate::impl_active;
impl SclyPropertyData for ActorKeyFrame<'_> {
    const OBJECT_TYPE: u8 = 0x1D;
    impl_active!();
}
