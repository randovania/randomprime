use auto_struct_macros::auto_struct;
use reader_writer::{CStr, CStrConversionExtension};

use crate::{res_id::*, ResId, SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct AreaAttributes<'r> {
    #[auto_struct(expect = 9)]
    pub prop_count: u32,

    // AreaAttributes is the one SCLY object with no name property. This placeholder is never
    // serialized; it only satisfies the name accessors that build_scly_property generates.
    #[auto_struct(literal = b"\0".as_cstr())]
    pub name: CStr<'r>,

    // ScriptLoader::LoadAreaAttributes returns nullptr unless this is exactly 1
    pub load: u32,

    pub show_skybox: u8,
    pub fx_type: u32,
    pub env_fx_density: f32,
    pub thermal_heat: f32,
    pub xray_fog_distance: f32,
    pub world_lighting_level: f32,
    pub skybox: ResId<CMDL>,
    pub phazon_type: u32,
}

impl SclyPropertyData for AreaAttributes<'_> {
    const OBJECT_TYPE: u8 = 0x4E;
}
