use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    impl_active, impl_position, impl_rotation, res_id::*, scly_props::structs::DamageInfo, ResId,
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct BeamInfo {
    pub unknown: u32,
    pub beam_attributes: u32,
    pub contact_fx: ResId<PART>,
    pub pulse_fx: ResId<PART>,
    pub texture: ResId<TXTR>,
    pub glow_texture: ResId<TXTR>,
    pub length: f32,
    pub radius: f32,
    pub expansion_speed: f32,
    pub life_time: f32,
    pub pulse_speed: f32,
    pub shutdown_time: f32,
    pub contact_fx_scale: f32,
    pub pulse_fx_scale: f32,
    pub travel_speed: f32,
    pub inner_color: GenericArray<f32, U4>,
    pub outer_color: GenericArray<f32, U4>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptBeam<'r> {
    #[auto_struct(expect = 7)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub weapon_description: ResId<WPSC>,
    pub beam_info: BeamInfo,
    pub damage_info: DamageInfo,
}

impl SclyPropertyData for ScriptBeam<'_> {
    const OBJECT_TYPE: u8 = 0x81;

    impl_active!();
    impl_position!();
    impl_rotation!();
}
