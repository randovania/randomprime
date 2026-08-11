use crate::SclyPropertyData;
use auto_struct_macros::auto_struct;
use reader_writer::CStr;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct TeamAIMgr<'r> {
    #[auto_struct(expect = 10)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub ai_count: u32,
    pub melee_count: u32,
    pub ranged_count: u32,
    pub unknown_count: u32,
    pub max_melee_attacker_count: u32,
    pub max_ranged_attacker_count: u32,
    pub position_mode: u32,
    pub melee_time_interval: f32,
    pub ranged_time_interval: f32,
}

impl SclyPropertyData for TeamAIMgr<'_> {
    const OBJECT_TYPE: u8 = 0x6C;
}
