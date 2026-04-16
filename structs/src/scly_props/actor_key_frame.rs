use auto_struct_macros::auto_struct;
use reader_writer::CStr;

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ActorKeyFrame<'r> {
    #[auto_struct(expect = 7)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub animation_id: u32,
    pub looping: u8,
    pub lifetime: f32,
    pub active: u8,
    pub fade_out: f32,
    pub total_playback: f32,
}

impl SclyPropertyData for ActorKeyFrame<'_> {
    const OBJECT_TYPE: u8 = 0x1D;
}
