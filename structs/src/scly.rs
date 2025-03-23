use std::{borrow::Cow, fmt, io};

use auto_struct_macros::auto_struct;
use reader_writer::{
    generic_array::GenericArray, typenum::*, FourCC, LCow, LazyArray, Readable, Reader, RoArray,
    Writable,
};

use crate::{
    scly_props,
    scly_structs::{DamageInfo, DamageVulnerability, HealthInfo, PatternedInfo},
};

#[macro_export]
macro_rules! impl_position {
    () => {
        const SUPPORTS_POSITION: bool = true;

        fn impl_get_position(&self) -> GenericArray<f32, U3> {
            self.position
        }

        fn impl_set_position(&mut self, x: GenericArray<f32, U3>) {
            self.position = x;
        }
    };
}

#[macro_export]
macro_rules! impl_rotation {
    () => {
        const SUPPORTS_ROTATION: bool = true;

        fn impl_get_rotation(&self) -> GenericArray<f32, U3> {
            self.rotation
        }

        fn impl_set_rotation(&mut self, x: GenericArray<f32, U3>) {
            self.rotation = x;
        }
    };
}

#[macro_export]
macro_rules! impl_scale {
    () => {
        const SUPPORTS_SCALE: bool = true;

        fn impl_get_scale(&self) -> GenericArray<f32, U3> {
            self.scale
        }

        fn impl_set_scale(&mut self, x: GenericArray<f32, U3>) {
            self.scale = x;
        }
    };
}

#[macro_export]
macro_rules! impl_patterned_info {
    () => {
        const SUPPORTS_PATTERNED_INFOS: bool = true;

        fn impl_get_patterned_infos(&self) -> Vec<PatternedInfo> {
            vec![self.patterned_info.clone()]
        }

        fn impl_set_patterned_infos(&mut self, x: Vec<PatternedInfo>) {
            self.patterned_info = x[0].clone();
        }
    };
}

#[macro_export]
macro_rules! impl_patterned_info_with_auxillary {
    () => {
        const SUPPORTS_PATTERNED_INFOS: bool = true;

        fn impl_get_patterned_infos(&self) -> Vec<PatternedInfo> {
            vec![self.patterned_info.clone()]
        }

        fn impl_set_patterned_infos(&mut self, x: Vec<PatternedInfo>) {
            self.patterned_info = x[0].clone();
        }

        const SUPPORTS_DAMAGE_INFOS: bool = true;

        fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
            vec![self.patterned_info.contact_damage.clone()]
        }

        fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
            self.patterned_info.contact_damage = x[0].clone();
        }

        const SUPPORTS_VULNERABILITIES: bool = true;

        fn impl_get_vulnerabilities(&self) -> Vec<DamageVulnerability> {
            vec![self.patterned_info.damage_vulnerability.clone()]
        }

        fn impl_set_vulnerabilities(&mut self, x: Vec<DamageVulnerability>) {
            self.patterned_info.damage_vulnerability = x[0].clone();
        }

        const SUPPORTS_HEALTH_INFOS: bool = true;

        fn impl_get_health_infos(&self) -> Vec<HealthInfo> {
            vec![self.patterned_info.health_info.clone()]
        }

        fn impl_set_health_infos(&mut self, x: Vec<HealthInfo>) {
            self.patterned_info.health_info = x[0].clone();
        }
    };
}

// damage_infos handled case-by-case
// vulnerabilities handled case-by-case

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Scly<'r> {
    #[auto_struct(expect = FourCC::from_bytes(b"SCLY"))]
    magic: FourCC,

    pub unknown: u32,

    #[auto_struct(derive = layers.len() as u32)]
    layer_count: u32,

    #[auto_struct(derive_from_iter = layers.iter()
            .map(&|i: LCow<SclyLayer>| i.size() as u32))]
    #[auto_struct(init = (layer_count as usize, ()))]
    _layer_sizes: RoArray<'r, u32>,

    #[auto_struct(init = (layer_count as usize, ()))]
    pub layers: LazyArray<'r, SclyLayer<'r>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct SclyLayer<'r> {
    pub unknown: u8,

    #[auto_struct(derive = objects.len() as u32)]
    object_count: u32,
    // TODO: Consider using DiffList here. Maybe requires profiling to decide...
    #[auto_struct(init = (object_count as usize, ()))]
    pub objects: LazyArray<'r, SclyObject<'r>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

impl Default for SclyLayer<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'r> SclyLayer<'r> {
    pub fn new() -> SclyLayer<'r> {
        SclyLayer {
            unknown: 0,
            objects: vec![].into(),
        }
    }
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct SclyObject<'r> {
    #[auto_struct(derive = property_data.object_type())]
    object_type: u8,

    #[auto_struct(derive = (8 + connections.size() + property_data.size()) as u32)]
    instance_size: u32,

    pub instance_id: u32,

    #[auto_struct(derive = connections.len() as u32)]
    connection_count: u32,
    #[auto_struct(init = (connection_count as usize, ()))]
    pub connections: LazyArray<'r, Connection>,

    #[auto_struct(init = (object_type, (instance_size - 8) as usize - connections.size()))]
    pub property_data: SclyProperty<'r>,
}

impl SclyObject<'_> {
    pub fn get_layer_idx(self) -> usize {
        ((self.instance_id >> 26) & 0x3F) as usize
    }

    pub fn get_area_idx(self) -> usize {
        ((self.instance_id >> 16) & 0xFF) as usize
    }

    pub fn get_object_idx(self) -> usize {
        (self.instance_id & 0xFFFF) as usize
    }
}

macro_rules! build_scly_property {
    ($($name:ident, $is_check:ident, $accessor:ident, $accessor_mut:ident,)*) => {

        #[derive(Clone, Debug)]
        pub enum SclyProperty<'r>
        {
            Unknown {
                object_type: u8,
                data: Reader<'r>
            },

            $($name(Box<scly_props::$name<'r >> ),)*
        }

        impl<'r> SclyProperty<'r>
        {
            pub fn object_type(&self) -> u8
            {
                match *self {
                    SclyProperty::Unknown { object_type, .. } => object_type,
                    $(SclyProperty::$name(_) =>
                      <scly_props::$name as SclyPropertyData>::OBJECT_TYPE,)*
                }
            }

            /* Position */

            pub fn supports_position(&self) -> bool {
                let object_type = self.object_type();
                #[allow(unreachable_patterns)] // ridley throws a warning because we have both PAL and NTSC ridley definitions
                match object_type {
                    $(<scly_props::$name as SclyPropertyData>::OBJECT_TYPE => <scly_props::$name as SclyPropertyData>::SUPPORTS_POSITION,)*
                    _ => false,
                }
            }

            pub fn get_position(&mut self) -> [f32;3]
            {
                self.guess_kind(); // TODO: shouldn't need mutability for read
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support position (get)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            let prop = self.$accessor();
                            prop.unwrap().impl_get_position().into()
                        },
                    )*
                }
            }

            pub fn set_position(&mut self, pos: [f32;3])
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support position (set)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            self.$accessor_mut().unwrap().impl_set_position(pos.into());
                        },
                    )*
                }
            }

            /* Rotation */

            pub fn supports_rotation(&self) -> bool {
                let object_type = self.object_type();
                #[allow(unreachable_patterns)] // ridley throws a warning because we have both PAL and NTSC ridley definitions
                match object_type {
                    $(<scly_props::$name as SclyPropertyData>::OBJECT_TYPE => <scly_props::$name as SclyPropertyData>::SUPPORTS_ROTATION,)*
                    _ => false,
                }
            }

            pub fn get_rotation(&mut self) -> [f32;3]
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support rotation (get)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            let prop = self.$accessor();
                            prop.unwrap().impl_get_rotation().into()
                        },
                    )*
                }
            }

            pub fn set_rotation(&mut self, pos: [f32;3])
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support rotation (set)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            self.$accessor_mut().unwrap().impl_set_rotation(pos.into());
                        },
                    )*
                }
            }

            /* Scale */

            pub fn supports_scale(&self) -> bool {
                let object_type = self.object_type();
                #[allow(unreachable_patterns)] // ridley throws a warning because we have both PAL and NTSC ridley definitions
                match object_type {
                    $(<scly_props::$name as SclyPropertyData>::OBJECT_TYPE => <scly_props::$name as SclyPropertyData>::SUPPORTS_SCALE,)*
                    _ => false,
                }
            }

            pub fn get_scale(&mut self) -> [f32;3]
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support scale (get)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            let prop = self.$accessor();
                            prop.unwrap().impl_get_scale().into()
                        },
                    )*
                }
            }

            pub fn set_scale(&mut self, pos: [f32;3])
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support scale (set)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            self.$accessor_mut().unwrap().impl_set_scale(pos.into());
                        },
                    )*
                }
            }

            /* Patterned Info */

            pub fn supports_patterned_infos(&self) -> bool {
                let object_type = self.object_type();
                #[allow(unreachable_patterns)] // ridley throws a warning because we have both PAL and NTSC ridley definitions
                match object_type {
                    $(<scly_props::$name as SclyPropertyData>::OBJECT_TYPE => <scly_props::$name as SclyPropertyData>::SUPPORTS_PATTERNED_INFOS,)*
                    _ => false,
                }
            }

            pub fn get_patterned_infos(&mut self) -> Vec<PatternedInfo>
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support patterned_infos (get)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            let prop = self.$accessor();
                            prop.unwrap().impl_get_patterned_infos()
                        },
                    )*
                }
            }

            pub fn set_patterned_infos(&mut self, x: Vec<PatternedInfo>)
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support patterned_infos (set)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            self.$accessor_mut().unwrap().impl_set_patterned_infos(x);
                        },
                    )*
                }
            }

            /* Damage Infos */

            pub fn supports_damage_infos(&self) -> bool {
                let object_type = self.object_type();
                #[allow(unreachable_patterns)] // ridley throws a warning because we have both PAL and NTSC ridley definitions
                match object_type {
                    $(<scly_props::$name as SclyPropertyData>::OBJECT_TYPE => <scly_props::$name as SclyPropertyData>::SUPPORTS_DAMAGE_INFOS,)*
                    _ => false,
                }
            }

            pub fn get_damage_infos(&mut self) -> Vec<DamageInfo>
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support damage_infos (get)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            let prop = self.$accessor();
                            prop.unwrap().impl_get_damage_infos()
                        },
                    )*
                }
            }

            pub fn set_damage_infos(&mut self, x: Vec<DamageInfo>)
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support damage_infos (set)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            self.$accessor_mut().unwrap().impl_set_damage_infos(x);
                        },
                    )*
                }
            }

            /* Vulnerabilities */

            pub fn supports_vulnerabilities(&self) -> bool {
                let object_type = self.object_type();
                #[allow(unreachable_patterns)] // ridley throws a warning because we have both PAL and NTSC ridley definitions
                match object_type {
                    $(<scly_props::$name as SclyPropertyData>::OBJECT_TYPE => <scly_props::$name as SclyPropertyData>::SUPPORTS_VULNERABILITIES,)*
                    _ => false,
                }
            }

            pub fn get_vulnerabilities(&mut self) -> Vec<DamageVulnerability>
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support vulnerabilities (get)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            let prop = self.$accessor();
                            prop.unwrap().impl_get_vulnerabilities()
                        },
                    )*
                }
            }

            pub fn set_vulnerabilities(&mut self, x: Vec<DamageVulnerability>)
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support vulnerabilities (set)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            self.$accessor_mut().unwrap().impl_set_vulnerabilities(x);
                        },
                    )*
                }
            }

            /* Health Infos */

            pub fn supports_health_infos(&self) -> bool {
                let object_type = self.object_type();
                #[allow(unreachable_patterns)] // ridley throws a warning because we have both PAL and NTSC ridley definitions
                match object_type {
                    $(<scly_props::$name as SclyPropertyData>::OBJECT_TYPE => <scly_props::$name as SclyPropertyData>::SUPPORTS_HEALTH_INFOS,)*
                    _ => false,
                }
            }

            pub fn get_health_infos(&mut self) -> Vec<HealthInfo>
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support health infos (get)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            let prop = self.$accessor();
                            prop.unwrap().impl_get_health_infos()
                        },
                    )*
                }
            }

            pub fn set_health_infos(&mut self, x: Vec<HealthInfo>)
            {
                self.guess_kind();
                match *self {
                    SclyProperty::Unknown { object_type, .. } => panic!("0x{:X} doesn't support health infos (set)", object_type),
                    $(
                        SclyProperty::$name(_) => {
                            self.$accessor_mut().unwrap().impl_set_health_infos(x);
                        },
                    )*
                }
            }

            pub fn guess_kind(&mut self)
            {
                let (mut reader, object_type) = match *self {
                    SclyProperty::Unknown { ref data, object_type }
                        => (data.clone(), object_type),
                    _ => return,
                };

                let old_len = self.size();

                *self = if false {
                    return
                } $(else if object_type == <scly_props::$name as SclyPropertyData>::OBJECT_TYPE {
                    SclyProperty::$name(reader.read(()))
                })* else {
                    return
                };

                if self.size() != old_len {
                    if self.size() < old_len {
                        panic!("scly object type=0x{:X} was an unexpected size. We expected {} bytes, in reality the object is {} bytes.\nFix this by adding the following line to the defining struct:\n    pub dont_care: GenericArray<u8, U{}>,\n", object_type, self.size(), old_len, old_len - self.size());
                    } else {
                        panic!("scly object type=0x{:X} was an unexpected size. We expected {} bytes, in reality the object is {} bytes.", object_type, self.size(), old_len);
                    }
                }
            }

            $(
                pub fn $is_check(&self) -> bool
                {
                    match *self {
                        SclyProperty::$name(_) => true,
                        SclyProperty::Unknown { object_type, .. } =>
                            object_type == <scly_props::$name as SclyPropertyData>::OBJECT_TYPE,
                        _ => false,
                    }
                }

                pub fn $accessor(&self) -> Option<Cow<scly_props::$name<'r>>>
                {
                    match *self {
                        SclyProperty::$name(ref inst) => Some(Cow::Borrowed(inst)),
                        SclyProperty::Unknown { ref data, object_type, .. } => {
                            if object_type == <scly_props::$name as SclyPropertyData>::OBJECT_TYPE {
                                Some(Cow::Owned(data.clone().read(())))
                            } else {
                                None
                            }
                        },
                        _ => None,
                    }
                }

                pub fn $accessor_mut(&mut self) -> Option<&mut scly_props::$name<'r>>
                {
                    let (mut data, object_type) = match *self {
                        SclyProperty::Unknown { ref data, object_type, .. } =>
                            (data.clone(), object_type),
                        SclyProperty::$name(ref mut inst) => return Some(inst),
                        _ => return None,
                    };
                    if object_type != <scly_props::$name as SclyPropertyData>::OBJECT_TYPE {
                        return None
                    }
                    *self = SclyProperty::$name(data.read(()));

                    match *self {
                        SclyProperty::$name(ref mut inst) => return Some(inst),
                        _ => panic!(),
                    }
                }
            )*
        }

        impl<'r> Readable<'r> for SclyProperty<'r>
        {
            type Args = (u8, usize);
            fn read_from(reader: &mut Reader<'r>, (otype, size): Self::Args) -> Self
            {
                let prop = SclyProperty::Unknown {
                    object_type: otype,
                    data: reader.truncated(size),
                };
                reader.advance(size);
                prop
            }

            fn size(&self) -> usize
            {
                match *self {
                    SclyProperty::Unknown { ref data, .. } => data.len(),
                    $(SclyProperty::$name(ref i) => i.size(),)*
                }
            }
        }

        impl<'r> Writable for SclyProperty<'r>
        {
            fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
            {
                match *self {
                    SclyProperty::Unknown { ref data, .. } => {
                        writer.write_all(&data)?;
                        Ok(data.len() as u64)
                    },
                    $(SclyProperty::$name(ref i) => i.write_to(writer),)*
                }
            }
        }

        $(
        impl<'r> From<scly_props::$name<'r>> for SclyProperty<'r>
        {
            fn from(prop: scly_props::$name<'r>) -> SclyProperty<'r>
            {
                SclyProperty::$name(Box::new(prop))
            }
        }
        )*

    };
}

build_scly_property!(
    Actor,
    is_actor,
    as_actor,
    as_actor_mut,
    ActorKeyFrame,
    is_actor_key_frame,
    as_actor_key_frame,
    as_actor_key_frame_mut,
    ActorRotate,
    is_actor_rotate,
    as_actor_rotate,
    as_actor_rotate_mut,
    BallTrigger,
    is_ball_trigger,
    as_ball_trigger,
    as_ball_trigger_mut,
    Camera,
    is_camera,
    as_camera,
    as_camera_mut,
    CameraBlurKeyframe,
    is_camera_blur_keyframe,
    as_camera_blur_keyframe,
    as_camera_blur_keyframe_mut,
    CameraFilterKeyframe,
    is_camera_filter_keyframe,
    as_camera_filter_keyframe,
    as_camera_filter_keyframe_mut,
    CameraHint,
    is_camera_hint,
    as_camera_hint,
    as_camera_hint_mut,
    CameraHintTrigger,
    is_camera_hint_trigger,
    as_camera_hint_trigger,
    as_camera_hint_trigger_mut,
    ControllerAction,
    is_controller_action,
    as_controller_action,
    as_controller_action_mut,
    Counter,
    is_counter,
    as_counter,
    as_counter_mut,
    DamageableTrigger,
    is_damageable_trigger,
    as_damageable_trigger,
    as_damageable_trigger_mut,
    DistanceFog,
    is_distance_fog,
    as_distance_fog,
    as_distance_fog_mut,
    Dock,
    is_dock,
    as_dock,
    as_dock_mut,
    Door,
    is_door,
    as_door,
    as_door_mut,
    Effect,
    is_effect,
    as_effect,
    as_effect_mut,
    GrapplePoint,
    is_grapple_point,
    as_grapple_point,
    as_grapple_point_mut,
    HudMemo,
    is_hud_memo,
    as_hud_memo,
    as_hud_memo_mut,
    MemoryRelay,
    is_memory_relay,
    as_memory_relay,
    as_memory_relay_mut,
    NewCameraShaker,
    is_new_camera_shaker,
    as_new_camera_shaker,
    as_new_camera_shaker_mut,
    Pickup,
    is_pickup,
    as_pickup,
    as_pickup_mut,
    PickupGenerator,
    is_pickup_generator,
    as_pickup_generator,
    as_pickup_generator_mut,
    Platform,
    is_platform,
    as_platform,
    as_platform_mut,
    PlayerActor,
    is_player_actor,
    as_player_actor,
    as_player_actor_mut,
    PlayerHint,
    is_player_hint,
    as_player_hint,
    as_player_hint_mut,
    PointOfInterest,
    is_point_of_interest,
    as_point_of_interest,
    as_point_of_interest_mut,
    Relay,
    is_relay,
    as_relay,
    as_relay_mut,
    SnakeWeedSwarm,
    is_snake_weed_swarm,
    as_snake_weed_swarm,
    as_snake_weed_swarm_mut,
    Sound,
    is_sound,
    as_sound,
    as_sound_mut,
    SpawnPoint,
    is_spawn_point,
    as_spawn_point,
    as_spawn_point_mut,
    SpecialFunction,
    is_special_function,
    as_special_function,
    as_special_function_mut,
    SpiderBallWaypoint,
    is_spider_ball_waypoint,
    as_spider_ball_waypoint,
    as_spider_ball_waypoint_mut,
    StreamedAudio,
    is_streamed_audio,
    as_streamed_audio,
    as_streamed_audio_mut,
    Switch,
    is_switch,
    as_switch,
    as_switch_mut,
    Timer,
    is_timer,
    as_timer,
    as_timer_mut,
    Trigger,
    is_trigger,
    as_trigger,
    as_trigger_mut,
    Waypoint,
    is_waypoint,
    as_waypoint,
    as_waypoint_mut,
    Water,
    is_water,
    as_water,
    as_water_mut,
    WorldLightFader,
    is_world_light_fader,
    as_world_light_fader,
    as_world_light_fader_mut,
    WorldTransporter,
    is_world_transporter,
    as_world_transporter,
    as_world_transporter_mut,
    // bosses
    Beetle,
    is_beetle,
    as_beetle,
    as_beetle_mut,
    Drone,
    is_drone,
    as_drone,
    as_drone_mut,
    NewIntroBoss,
    is_new_intro_boss,
    as_new_intro_boss,
    as_new_intro_boss_mut,
    ActorContraption,
    is_actor_contraption,
    as_actor_contraption,
    as_actor_contraption_mut,
    Flaahgra,
    is_flaahgra,
    as_flaahgra,
    as_flaahgra_mut,
    IceSheegoth,
    is_ice_sheegoth,
    as_ice_sheegoth,
    as_ice_sheegoth_mut,
    Thardus,
    is_thardus,
    as_thardus,
    as_thardus_mut,
    ElitePirate,
    is_elite_pirate,
    as_elite_pirate,
    as_elite_pirate_mut,
    OmegaPirate,
    is_omega_pirate,
    as_omega_pirate,
    as_omega_pirate_mut,
    RidleyV1,
    is_ridley_v1,
    as_ridley_v1,
    as_ridley_v1_mut,
    RidleyV2,
    is_ridley_v2,
    as_ridley_v2,
    as_ridley_v2_mut,
    MetroidPrimeStage1,
    is_metroidprimestage1,
    as_metroidprimestage1,
    as_metroidprimestage1_mut,
    MetroidPrimeStage2,
    is_metroidprimestage2,
    as_metroidprimestage2,
    as_metroidprimestage2_mut,
    // "Generic" edit update
    AIJumpPoint,
    is_ai_jump_point,
    as_ai_jump_point,
    as_ai_jump_point_mut,
    AmbientAI,
    is_ambient_ai,
    as_ambient_ai,
    as_ambient_ai_mut,
    AtomicAlpha,
    is_atomic_alpha,
    as_atomic_alpha,
    as_atomic_alpha_mut,
    AtomicBeta,
    is_atomic_beta,
    as_atomic_beta,
    as_atomic_beta_mut,
    Babygoth,
    is_babygoth,
    as_babygoth,
    as_babygoth_mut,
    Bloodflower,
    is_bloodflower,
    as_bloodflower,
    as_bloodflower_mut,
    Burrower,
    is_burrower,
    as_burrower,
    as_burrower_mut,
    CameraPitchVolume,
    is_camera_pitch_volume,
    as_camera_pitch_volume,
    as_camera_pitch_volume_mut,
    CameraWaypoint,
    is_camera_waypoint,
    as_camera_waypoint,
    as_camera_waypoint_mut,
    ChozoGhost,
    is_chozo_ghost,
    as_chozo_ghost,
    as_chozo_ghost_mut,
    CoverPoint,
    is_cover_point,
    as_cover_point,
    as_cover_point_mut,
    Debris,
    is_debris,
    as_debris,
    as_debris_mut,
    DebrisExtended,
    is_debris_extended,
    as_debris_extended,
    as_debris_extended_mut,
    EnergyBall,
    is_energy_ball,
    as_energy_ball,
    as_energy_ball_mut,
    Eyeball,
    is_eyeball,
    as_eyeball,
    as_eyeball_mut,
    FireFlea,
    is_fire_flea,
    as_fire_flea,
    as_fire_flea_mut,
    FishCloud,
    is_fish_cloud,
    as_fish_cloud,
    as_fish_cloud_mut,
    FlaahgraTentacle,
    is_flaahgra_tentacle,
    as_flaahgra_tentacle,
    as_flaahgra_tentacle_mut,
    FlickerBat,
    is_flicker_bat,
    as_flicker_bat,
    as_flicker_bat_mut,
    FlyingPirate,
    is_flying_pirate,
    as_flying_pirate,
    as_flying_pirate_mut,
    Geemer,
    is_geemer,
    as_geemer,
    as_geemer_mut,
    GunTurret,
    is_gun_turret,
    as_gun_turret,
    as_gun_turret_mut,
    JellyZap,
    is_jelly_zap,
    as_jelly_zap,
    as_jelly_zap_mut,
    Magdolite,
    is_magdolite,
    as_magdolite,
    as_magdolite_mut,
    Metaree,
    is_metaree,
    as_metaree,
    as_metaree_mut,
    Metroid,
    is_metroid,
    as_metroid,
    as_metroid_mut,
    MetroidBeta,
    is_metroid_beta,
    as_metroid_beta,
    as_metroid_beta_mut,
    Parasite,
    is_parasite,
    as_parasite,
    as_parasite_mut,
    PhazonHealingNodule,
    is_phazon_healing_nodule,
    as_phazon_healing_nodule,
    as_phazon_healing_nodule_mut,
    PhazonPool,
    is_phazon_pool,
    as_phazon_pool,
    as_phazon_pool_mut,
    PuddleSpore,
    is_puddle_spore,
    as_puddle_spore,
    as_puddle_spore_mut,
    PuddleToadGamma,
    is_puddle_toad_gamma,
    as_puddle_toad_gamma,
    as_puddle_toad_gamma_mut,
    Puffer,
    is_puffer,
    as_puffer,
    as_puffer_mut,
    Ripper,
    is_ripper,
    as_ripper,
    as_ripper_mut,
    Seedling,
    is_seedling,
    as_seedling,
    as_seedling_mut,
    SpacePirate,
    is_space_pirate,
    as_space_pirate,
    as_space_pirate_mut,
    SpankWeed,
    is_spank_weed,
    as_spank_weed,
    as_spank_weed_mut,
    ThardusRockProjectile,
    is_thardus_rock_projectile,
    as_rock_projectile,
    as_rock_projectile_mut,
    Tryclops,
    is_tryclops,
    as_tryclops,
    as_tryclops_mut,
    WarWasp,
    is_war_wasp,
    as_war_wasp,
    as_war_wasp_mut,
);

pub trait SclyPropertyData {
    const OBJECT_TYPE: u8;

    /* Position */
    const SUPPORTS_POSITION: bool = false;

    fn impl_get_position(&self) -> GenericArray<f32, U3> {
        panic!(
            "Script object type 0x{:X} does not implement the 'position' property",
            Self::OBJECT_TYPE
        )
    }

    fn impl_set_position(&mut self, _: GenericArray<f32, U3>) {
        panic!(
            "Script object type 0x{:X} does not implement the 'position' property",
            Self::OBJECT_TYPE
        )
    }

    /* Rotation */
    const SUPPORTS_ROTATION: bool = false;

    fn impl_get_rotation(&self) -> GenericArray<f32, U3> {
        panic!(
            "Script object type 0x{:X} does not implement the 'rotation' property",
            Self::OBJECT_TYPE
        )
    }

    fn impl_set_rotation(&mut self, _: GenericArray<f32, U3>) {
        panic!(
            "Script object type 0x{:X} does not implement the 'rotation' property",
            Self::OBJECT_TYPE
        )
    }

    /* Scale */
    const SUPPORTS_SCALE: bool = false;

    fn impl_get_scale(&self) -> GenericArray<f32, U3> {
        panic!(
            "Script object type 0x{:X} does not implement the 'scale' property",
            Self::OBJECT_TYPE
        )
    }

    fn impl_set_scale(&mut self, _: GenericArray<f32, U3>) {
        panic!(
            "Script object type 0x{:X} does not implement the 'scale' property",
            Self::OBJECT_TYPE
        )
    }

    /* Patterned Infos */
    const SUPPORTS_PATTERNED_INFOS: bool = false;

    fn impl_get_patterned_infos(&self) -> Vec<PatternedInfo> {
        panic!(
            "Script object type 0x{:X} does not implement the 'Patterned Info' property",
            Self::OBJECT_TYPE
        )
    }

    fn impl_set_patterned_infos(&mut self, _: Vec<PatternedInfo>) {
        panic!(
            "Script object type 0x{:X} does not implement the 'Patterned Info' property",
            Self::OBJECT_TYPE
        )
    }

    /* Damage Infos */
    const SUPPORTS_DAMAGE_INFOS: bool = false;

    fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
        panic!(
            "Script object type 0x{:X} does not implement the 'Damage Infos' property",
            Self::OBJECT_TYPE
        )
    }

    fn impl_set_damage_infos(&mut self, _: Vec<DamageInfo>) {
        panic!(
            "Script object type 0x{:X} does not implement the 'Damage Infos' property",
            Self::OBJECT_TYPE
        )
    }

    /* Vulnerabilities */
    const SUPPORTS_VULNERABILITIES: bool = false;

    fn impl_get_vulnerabilities(&self) -> Vec<DamageVulnerability> {
        panic!(
            "Script object type 0x{:X} does not implement the 'Vulnerabilities' property",
            Self::OBJECT_TYPE
        )
    }

    fn impl_set_vulnerabilities(&mut self, _: Vec<DamageVulnerability>) {
        panic!(
            "Script object type 0x{:X} does not implement the 'Vulnerabilities' property",
            Self::OBJECT_TYPE
        )
    }

    /* Health Infos */
    const SUPPORTS_HEALTH_INFOS: bool = false;

    fn impl_get_health_infos(&self) -> Vec<HealthInfo> {
        panic!(
            "Script object type 0x{:X} does not implement the 'Vulnerabilities' property",
            Self::OBJECT_TYPE
        )
    }

    fn impl_set_health_infos(&mut self, _: Vec<HealthInfo>) {
        panic!(
            "Script object type 0x{:X} does not implement the 'Vulnerabilities' property",
            Self::OBJECT_TYPE
        )
    }
}

#[auto_struct(Readable, FixedSize, Writable)]
#[derive(Debug, Clone)]
pub struct Connection {
    pub state: ConnectionState,
    pub message: ConnectionMsg,
    pub target_object_id: u32,
}

macro_rules! build_scly_conn_field {
    ($struct_name:ident { $($field:ident = $value:expr,)+ }) => {
        impl $struct_name
        {
            $(pub const $field: $struct_name = $struct_name($value);)+
        }

        impl fmt::Debug for $struct_name
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
            {
                match self.0 {
                    $(
                    $value => f.write_fmt(format_args!("{}::{}", stringify!($struct_name), stringify!($field))),
                    )+
                    n => f.write_fmt(format_args!("{}(0x{:x})", stringify!($struct_name), n)),
                }
            }
        }

        impl<'r> Readable<'r> for $struct_name
        {
            type Args = ();

            fn read_from(reader: &mut Reader<'r>, (): Self::Args) -> Self
            {
                let i = reader.read(());
                $struct_name(i)
            }

            fn fixed_size() -> Option<usize>
            {
                u32::fixed_size()
            }
        }

        impl Writable for $struct_name
        {
            fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
            {
                self.0.write_to(writer)
            }
        }
    };
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ConnectionState(pub u32);
build_scly_conn_field!(ConnectionState {
    ANY = 0xFFFFFFFF,
    ACTIVE = 0x0,
    ARRIVED = 0x1,
    CLOSED = 0x2,
    ENTERED = 0x3,
    EXITED = 0x4,
    INACTIVE = 0x5,
    INSIDE = 0x6,
    MAX_REACHED = 0x7,
    OPEN = 0x8,
    ZERO = 0x9,
    ATTACK = 0xA,
    RETREAT = 0xC,
    PATROL = 0xD,
    DEAD = 0xE,
    CAMERA_PATH = 0xF,
    CAMERA_TARGET = 0x10,
    DEACTIVATE_STATE = 0x11,
    PLAY = 0x12,
    MASSIVE_DEATH = 0x13,
    DEATH_RATTLE = 0x14,
    ABOUT_TO_MASSIVELY_DIE = 0x15,
    DAMAGE = 0x16,
    INVULN_DAMAGE = 0x17,
    MASSIVE_FROZEN_DEATH = 0x18,
    MODIFY = 0x19,
    SCAN_START = 0x1A,
    SCAN_PROCESSING = 0x1B,
    SCAN_DONE = 0x1C,
    UNFROZEN = 0x1D,
    DEFAULT = 0x1E,
    REFLECTED_DAMAGE = 0x1F,
    INHERIT_BOUNDS = 0x20,
});

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ConnectionMsg(pub u32);
build_scly_conn_field!(ConnectionMsg {
    NONE = 0xFFFFFFFF,
    UNKM0 = 0x0,
    ACTIVATE = 0x1,
    ARRIVED = 0x2,
    CLOSE = 0x3,
    DEACTIVATE = 0x4,
    DECREMENT = 0x5,
    FOLLOW = 0x6,
    INCREMENT = 0x7,
    NEXT = 0x8,
    OPEN = 0x9,
    RESET = 0xA,
    RESET_AND_START = 0xB,
    SET_TO_MAX = 0xC,
    SET_TO_ZERO = 0xD,
    START = 0xE,
    STOP = 0xF,
    STOP_AND_RESET = 0x10,
    TOGGLE_ACTIVE = 0x11,
    UNKM18 = 0x12,
    ACTION = 0x13,
    PLAY = 0x14,
    ALERT = 0x15,
    INTERNAL_MESSAGE00 = 0x16,
    ON_FLOOR = 0x17,
    INTERNAL_MESSAGE02 = 0x18,
    INTERNAL_MESSAGE03 = 0x19,
    FALLING = 0x1A,
    ON_ICE_SURFACE = 0x1B,
    ON_MUD_SLOW_SURFACE = 0x1C,
    ON_NORMAL_SURFACE = 0x1D,
    TOUCHED = 0x1E,
    ADD_PLATFORM_RIDER = 0x1F,
    LAND_ON_NOT_FLOOR = 0x20,
    REGISTERED = 0x21,
    DELETED = 0x22,
    INITIALIZED_IN_AREA = 0x23,
    WORLD_INITIALIZED = 0x24,
    ADD_SPLASH_INHABITANT = 0x25,
    UPDATE_SPLASH_INHABITANT = 0x26,
    REMOVE_SPLASH_INHABITANT = 0x27,
    JUMPED = 0x28,
    DAMAGE = 0x29,
    INVULN_DAMAGE = 0x2A,
    PROJECTILE_COLLIDE = 0x2B,
    IN_SNAKE_WEED = 0x2C,
    ADD_PHAZON_POOOL_INHABITANT = 0x2D,
    UPDATE_PHAZON_POOL_INHABITANT = 0x2E,
    REMOVE_PHAZON_POOL_INHABITANT = 0x2F,
    SUSPENDED_MOVE = 0x30,
});
