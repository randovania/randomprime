use auto_struct_macros::auto_struct;
use reader_writer::CStr;

use crate::{impl_active, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalHeatFader<'r> {
    #[auto_struct(expect = 4)]
    prop_count: u32,

    pub name: CStr<'r>,
    pub active: u8,
    pub faded_heat_level: f32,

    // PWE calls this "Initial Heat Level", but the engine passes it as CScriptDistanceFog's
    // thermal fade speed (ScriptLoader::LoadThermalHeatFader)
    pub fade_speed: f32,
}

impl SclyPropertyData for ThermalHeatFader<'_> {
    const OBJECT_TYPE: u8 = 0x7D;

    impl_active!();
}
