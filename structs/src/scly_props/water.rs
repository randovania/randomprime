use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{scly_props::structs::DamageInfo, SclyPropertyData};

// https://github.com/AxioDL/metaforce/blob/1655d229cfdfbd5f792a7c3e84adc862653f70a7/DataSpec/DNAMP1/ScriptObjects/Water.hpp
#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Water<'r> {
    #[auto_struct(expect = 63)]
    prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub damage_info: DamageInfo,
    pub unknown1: GenericArray<f32, U3>,
    pub unknown2: u32,
    pub unknown3: u8,
    pub display_fluid_surface: u8,
    pub txtr1: u32,
    pub txtr2: u32,
    pub txtr3: u32,
    pub txtr4: u32,
    pub refl_map_txtr: u32,
    pub txtr6: u32,
    pub unknown5: GenericArray<f32, U3>,
    pub unknown6: f32,
    pub morph_in_time: f32,
    pub morph_out_time: f32,
    pub active: u8,
    pub fluid_type: u32,
    pub unknown11: u8,
    pub unknown12: f32,
    pub fluid_uv_motion: FluidUVMotion,
    pub unknown30: f32,
    pub unknown31: f32,
    pub unknown32: f32,
    pub unknown33: f32,
    pub unknown34: f32,
    pub unknown35: f32,
    pub unknown36: f32,
    pub unknown37: f32,
    pub unknown38: GenericArray<f32, U4>, // RGBA
    pub unknown39: GenericArray<f32, U4>, // RGBA
    pub small_enter_part: u32,
    pub med_enter_part: u32,
    pub large_enter_part: u32,
    pub part4: u32,
    pub part5: u32,
    pub sound1: u32,
    pub sound2: u32,
    pub sound3: u32,
    pub sound4: u32,
    pub sound5: u32,
    pub unknown40: f32,
    pub unknown41: u32,
    pub unknown42: f32,
    pub unknown43: f32,
    pub unknown44: f32,
    pub unknown45: f32,
    pub unknown46: f32,
    pub unknown47: f32,
    pub heat_wave_height: f32,
    pub heat_wave_speed: f32,
    pub heat_wave_color: GenericArray<f32, U4>, // RGBA
    pub lightmap_txtr: u32,
    pub unknown51: f32,
    pub alpha_in_time: f32,
    pub alpha_out_time: f32,
    pub unknown54: u32,
    pub unknown55: u32,
    pub crash_the_game: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct FluidUVMotion {
    pub fluid_layer_motion1: FluidLayerMotion,
    pub fluid_layer_motion2: FluidLayerMotion,
    pub fluid_layer_motion3: FluidLayerMotion,
    pub unknown1: f32,
    pub unknown2: f32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct FluidLayerMotion {
    pub fluid_uv_motion: u32,
    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,
    pub unknown4: f32,
}

use crate::{impl_position, impl_scale};
impl SclyPropertyData for Water<'_> {
    const OBJECT_TYPE: u8 = 0x20;
    impl_position!();
    impl_scale!();

    const SUPPORTS_DAMAGE_INFOS: bool = true;

    fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
        vec![self.damage_info]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.damage_info = x[0];
    }
}
