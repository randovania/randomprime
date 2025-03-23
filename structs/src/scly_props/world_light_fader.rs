use auto_struct_macros::auto_struct;
use reader_writer::CStr;

use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct WorldLightFader<'r> {
    #[auto_struct(expect = 4)]
    prop_count: u32,

    pub name: CStr<'r>,
    pub active: u8,
    pub faded_light_level: f32,
    pub fade_speed: f32,
}

impl SclyPropertyData for WorldLightFader<'_> {
    const OBJECT_TYPE: u8 = 0x82;
}
