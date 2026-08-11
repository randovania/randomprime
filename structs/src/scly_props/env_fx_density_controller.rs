use crate::{impl_active, SclyPropertyData};
use auto_struct_macros::auto_struct;
use reader_writer::CStr;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct EnvFxDensityController<'r> {
    #[auto_struct(expect = 4)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub active: u8,
    pub density: f32,
    pub max_density_delta_speed: u32,
}

impl SclyPropertyData for EnvFxDensityController<'_> {
    const OBJECT_TYPE: u8 = 0x6A;

    impl_active!();
}
