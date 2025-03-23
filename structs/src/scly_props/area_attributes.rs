use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};
use std::marker::PhantomData;

use crate::{
    res_id::*,
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct AreaAttributes<'r> {
    #[auto_struct(expect = 9)]
    pub prop_count: u32,
    pub _a: PhantomData<&'r()>,

    pub load: u32, /* 0 causes the loader to bail and return null */
    pub skybox_enabled: u8,
    pub weather: u32, // none, snow, rain, bubbles
    pub env_fx_density: f32,
    pub thermal_heat: f32,
    pub xray_fog_distance: f32,
    pub world_lighting_level: f32,
    pub skybox: ResId<CMDL>,
    pub phazon_type: u32, // none, blue, orange
}

impl<'r> SclyPropertyData for AreaAttributes<'r> {
    const OBJECT_TYPE: u8 = 0x4E;
}
