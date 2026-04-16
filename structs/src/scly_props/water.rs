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
    pub force: GenericArray<f32, U3>,
    pub flags: u32,
    pub thermal_cold: u8,
    pub display_surface: u8,
    pub pattern_map1: u32,
    pub pattern_map2: u32,
    pub color_map: u32,
    pub bump_map: u32,
    pub env_map: u32,
    pub env_bump_map: u32,
    pub bump_light_dir: GenericArray<f32, U3>,
    pub bump_scale: f32,
    pub morph_in_time: f32,
    pub morph_out_time: f32,
    pub active: u8,
    pub fluid_type: u32,
    pub unknown: u8,
    pub alpha: f32,
    pub fluid_uv_motion: FluidUVMotion,
    pub turb_speed: f32,
    pub turb_distance: f32,
    pub turb_frequence_max: f32,
    pub turb_frequence_min: f32,
    pub turb_phase_max: f32,
    pub turb_phase_min: f32,
    pub turb_amplitude_max: f32,
    pub turb_amplitude_min: f32,
    pub splash_color: GenericArray<f32, U4>,     // RGBA
    pub inside_fog_color: GenericArray<f32, U4>, // RGBA
    pub small_enter_part: u32,
    pub med_enter_part: u32,
    pub large_enter_part: u32,
    pub visor_runoff_particle: u32,
    pub unmorph_visor_runoff_particle: u32,
    pub visor_runoff_sound: u32,
    pub unmorph_visor_runoff_sound: u32,
    pub splash_sfx1: u32,
    pub splash_sfx2: u32,
    pub splash_sfx3: u32,
    pub tile_size: f32,
    pub tile_subdivisions: u32,
    pub specular_min: f32,
    pub specular_max: f32,
    pub reflection_size: f32,
    pub ripple_intensity: f32,
    pub reflection_blend: f32,
    pub fog_bias: f32,
    pub fog_magnitude: f32,
    pub fog_speed: f32,
    pub fog_color: GenericArray<f32, U4>, // RGBA
    pub lightmap_txtr: u32,
    pub units_per_lightmap_texel: f32,
    pub alpha_in_time: f32,
    pub alpha_out_time: f32,
    pub alpha_in_recip: u32,
    pub alpha_out_recip: u32,
    pub crash_the_game: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct FluidUVMotion {
    pub fluid_layer_motion1: FluidLayerMotion,
    pub fluid_layer_motion2: FluidLayerMotion,
    pub fluid_layer_motion3: FluidLayerMotion,
    pub time_to_wrap: f32,
    pub orientation: f32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct FluidLayerMotion {
    pub fluid_uv_motion: u32,
    pub time_to_wrap: f32,
    pub orientation: f32,
    pub magnitude: f32,
    pub multiplication: f32,
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
