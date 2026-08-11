use auto_struct_macros::auto_struct;
use reader_writer::CStr;

use crate::{impl_active, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct RoomAcoustics<'r> {
    #[auto_struct(expect = 32)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub active: u8,
    pub vol_scale: u32,

    pub rev_hi: u8,
    pub rev_hi_dis: u8,
    pub rev_hi_coloration: f32,
    pub rev_hi_mix: f32,
    pub rev_hi_time: f32,
    pub rev_hi_damping: f32,
    pub rev_hi_pre_delay: f32,
    pub rev_hi_crosstalk: f32,

    pub chorus: u8,
    pub base_delay: f32,
    pub variation: f32,
    pub period: f32,

    pub rev_std: u8,
    pub rev_std_dis: u8,
    pub rev_std_coloration: f32,
    pub rev_std_mix: f32,
    pub rev_std_time: f32,
    pub rev_std_damping: f32,
    pub rev_std_pre_delay: f32,

    pub delay: u8,
    pub delay_l: u32,
    pub delay_r: u32,
    pub delay_s: u32,
    pub feedback_l: u32,
    pub feedback_r: u32,
    pub feedback_s: u32,
    pub output_l: u32,
    pub output_r: u32,
    pub output_s: u32,
}

impl SclyPropertyData for RoomAcoustics<'_> {
    const OBJECT_TYPE: u8 = 0x5D;

    impl_active!();
}
