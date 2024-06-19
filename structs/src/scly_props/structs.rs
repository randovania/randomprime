use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*};

use crate::{res_id::*, ResId};

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct GrappleParameters {
    #[auto_struct(expect = 12)]
    prop_count: u32,

    pub unknowns: GenericArray<f32, U11>,
    pub disable_turning: u8,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ActorParameters {
    #[auto_struct(expect = 14)]
    prop_count: u32,
    pub light_params: LightParameters,
    pub scan_params: ScannableParameters,

    pub xray_cmdl: ResId<CMDL>,
    pub xray_cskr: ResId<CSKR>,

    pub thermal_cmdl: ResId<CMDL>,
    pub thermal_cskr: ResId<CSKR>,

    pub unknown0: u8,
    pub unknown1: f32,
    pub unknown2: f32,

    pub visor_params: VisorParameters,

    pub enable_thermal_heat: u8,
    pub unknown3: u8,
    pub unknown4: u8,
    pub unknown5: f32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct AnimationParameters {
    pub animation_character_set: u32,
    pub character: u32,
    pub default_animation: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct AncsProp {
    pub file_id: ResId<ANCS>,
    pub node_index: u32,
    pub default_animation: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct LightParameters {
    #[auto_struct(expect = 14)]
    prop_count: u32,

    pub unknown0: u8,
    pub unknown1: f32,
    pub shadow_tessellation: u32,
    pub unknown2: f32,
    pub unknown3: f32,
    pub color: GenericArray<f32, U4>, // RGBA
    pub unknown4: u8,
    pub world_lighting: u32,
    pub light_recalculation: u32,
    pub unknown5: GenericArray<f32, U3>,
    pub unknown6: u32,
    pub unknown7: u32,
    pub unknown8: u8,
    pub light_layer_id: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ScannableParameters {
    #[auto_struct(expect = 1)]
    prop_count: u32,
    pub scan: ResId<SCAN>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct VisorParameters {
    #[auto_struct(expect = 3)]
    prop_count: u32,
    pub unknown0: u8,
    pub target_passthrough: u8,
    pub visor_mask: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Copy, Clone)]
pub struct DamageInfo {
    #[auto_struct(expect = 4)]
    prop_count: u32,
    pub weapon_type: u32,
    pub damage: f32,
    pub radius: f32,
    pub knockback_power: f32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct DamageVulnerability {
    #[auto_struct(expect = 18)]
    prop_count: u32,

    pub power: u32,
    pub ice: u32,
    pub wave: u32,
    pub plasma: u32,
    pub bomb: u32,
    pub power_bomb: u32,
    pub missile: u32,
    pub boost_ball: u32,
    pub phazon: u32,

    pub enemy_weapon0: u32,
    pub enemy_weapon1: u32,
    pub enemy_weapon2: u32,
    pub enemy_weapon3: u32,

    pub unknown_weapon0: u32,
    pub unknown_weapon1: u32,
    pub unknown_weapon2: u32,

    pub charged_beams: ChargedBeams,
    pub beam_combos: BeamCombos,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TypeVulnerability {
    Normal = 0x1,
    Reflect = 0x2,
    Immune = 0x3,
    DirectNormal = 0x5,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ChargedBeams {
    #[auto_struct(expect = 5)]
    prop_count: u32,

    pub power: u32,
    pub ice: u32,
    pub wave: u32,
    pub plasma: u32,
    pub phazon: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct BeamCombos {
    #[auto_struct(expect = 5)]
    prop_count: u32,

    pub power: u32,
    pub ice: u32,
    pub wave: u32,
    pub plasma: u32,
    pub phazon: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct HealthInfo {
    #[auto_struct(expect = 2)]
    prop_count: u32,

    pub health: f32,
    pub knockback_resistance: f32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct PatternedInfo {
    #[auto_struct(expect = 38)]
    prop_count: u32,

    pub mass: f32,
    pub speed: f32,
    pub turn_speed: f32,
    pub detection_range: f32,
    pub detection_height_range: f32,
    pub detection_angle: f32,
    pub min_attack_range: f32,
    pub max_attack_range: f32,
    pub average_attack_time: f32,
    pub attack_time_variation: f32,
    pub leash_radius: f32,
    pub player_leash_radius: f32,
    pub player_leash_time: f32,
    pub contact_damage: DamageInfo,
    pub damage_wait_time: f32,
    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,
    pub half_extent: f32,
    pub height: f32,
    pub body_origin: GenericArray<f32, U3>,
    pub step_up_height: f32,
    pub x_damage: f32,
    pub frozen_x_damage: f32,
    pub x_damage_delay: f32,
    pub death_sfx: u32,
    pub animation_parameters: AncsProp,
    pub active: u8,
    pub state_machine: ResId<AFSM>,
    pub into_freeze_dur: f32,
    pub out_of_freeze_dur: f32,
    pub unknown0: f32,
    pub pathfinding_index: u32,
    pub particle0_scale: GenericArray<f32, U3>,
    pub particle0: ResId<PART>,
    pub electric: ResId<ELSC>,
    pub particle1_scale: GenericArray<f32, U3>,
    pub particle1: ResId<PART>,
    pub ice_shatter_sfx: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct BeamInfo {
    #[auto_struct(expect = 16)]
    prop_count: u32,

    pub beam_attributes: u32,
    pub part1: u32,
    pub part2: u32,
    pub txtr1: u32,
    pub txtr2: u32,
    pub length: f32,
    pub radius: f32,
    pub expansion_speed: f32,
    pub lifetime: f32,
    pub pulse_speed: f32,
    pub shutdown_time: f32,
    pub contact_fx_scale: f32,
    pub pulse_fx_scale: f32,
    pub travel_speed: f32,
    pub inner_color: GenericArray<f32, U4>,
    pub outter_color: GenericArray<f32, U4>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct RidleyStruct1 {
    pub unknown0: u32,
    pub unknown1: u32,
    pub particles: GenericArray<ResId<PART>, U2>,
    pub textures: GenericArray<ResId<TXTR>, U2>,
    pub unknown2: f32,
    pub unknown3: f32,
    pub unknown4: f32,
    pub unknown5: f32,
    pub unknown6: f32,
    pub unknown7: f32,
    pub unknown8: f32,
    pub unknown9: f32,
    pub unknown10: f32,
    pub color0: GenericArray<f32, U4>,
    pub color1: GenericArray<f32, U4>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct RidleyStruct2 {
    pub unknown0: u32,
    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,
    pub unknown4: f32,
    pub unknown5: f32,
    pub unknown6: f32,
    pub unknown7: f32,
    pub unknown8: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraShakerComponent {
    pub unknown1: u32,
    pub unknown2: u8,
    pub am: CameraShakePoint,
    pub fm: CameraShakePoint,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraShakePoint {
    pub unknown1: u32,
    pub unknown2: u8,
    pub attack_time: f32,
    pub sustain_time: f32,
    pub duration: f32,
    pub magnitude: f32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct CameraHintParameters {
    #[auto_struct(expect = 22)]
    prop_count: u32,
    pub calculate_cam_pos: u8,
    pub chase_allowed: u8,
    pub boost_allowed: u8,
    pub obscure_avoidance: u8,
    pub volume_collider: u8,
    pub apply_immediately: u8,
    pub look_at_ball: u8,
    pub hint_distance_selection: u8,
    pub hint_distance_self_pos: u8,
    pub control_interpolation: u8,
    pub sinusoidal_interpolation: u8,
    pub sinusoidal_interpolation_hintless: u8,
    pub clamp_velocity: u8,
    pub skip_cinematic: u8,
    pub no_elevation_interp: u8,
    pub direct_elevation: u8,
    pub override_look_dir: u8,
    pub no_elevation_vel_clamp: u8,
    pub calculate_transform_from_prev_cam: u8,
    pub no_spline: u8,
    pub unknown21: u8,
    pub unknown22: u8,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct BoolFloat {
    pub override_flags: u8,
    pub value: f32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct BoolVec3 {
    pub override_flags: u8,
    pub value: GenericArray<f32, U3>,
}
