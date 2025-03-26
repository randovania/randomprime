use std::{
    collections::{HashMap, HashSet},
    fmt,
    fs::{self, File, OpenOptions},
    io::Read,
    str::FromStr,
};

use clap::{crate_version, App, Arg};
use json_data::*;
use json_strip::strip_jsonc_comments;
use reader_writer::{FourCC, Reader};
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize,
};
use structs::{res_id, MapaObjectVisibilityMode, ResId};

use crate::{
    custom_assets::custom_asset_ids, door_meta::DoorType, elevators::World,
    pickup_meta::PickupType, room_lookup::ROOM_BY_INTERNAL_ID, starting_items::StartingItems,
};

/*** Parsed Config (fn patch_iso) ***/

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum RunMode {
    CreateIso,
    ExportLogbook,
    ExportAssets,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub enum IsoFormat {
    Iso,
    Gcz,
    Ciso,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub enum ArtifactHintBehavior {
    Default,
    None,
    All,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub enum CutsceneMode {
    Original,
    Skippable,
    SkippableCompetitive,
    Competitive,
    Minor,
    Major,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub enum Visor {
    Combat,
    XRay,
    Scan,
    Thermal,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub enum Beam {
    Power,
    Ice,
    Wave,
    Plasma,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameBanner {
    pub game_name: Option<String>,
    pub game_name_full: Option<String>,
    pub developer: Option<String>,
    pub developer_full: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PickupConfig {
    pub id: Option<u32>,
    #[serde(alias = "type")]
    pub pickup_type: String,
    pub curr_increase: Option<i32>,
    pub max_increase: Option<i32>,
    pub model: Option<String>,
    pub scan_text: Option<String>,
    pub hudmemo_text: Option<String>,
    pub respawn: Option<bool>,
    pub position: Option<[f32; 3]>,
    pub modal_hudmemo: Option<bool>,
    pub jumbo_scan: Option<bool>,
    pub destination: Option<String>,
    pub show_icon: Option<bool>,
    pub invisible_and_silent: Option<bool>,
    pub thermal_only: Option<bool>,
    pub scale: Option<[f32; 3]>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScanConfig {
    pub id: Option<u32>,
    pub actor_id: Option<u32>,
    pub layer: Option<u32>,
    pub position: [f32; 3],
    pub combat_visible: Option<bool>,
    pub rotation: Option<f32>,
    pub is_red: Option<bool>,
    pub logbook_category: Option<u32>,
    pub logbook_title: Option<String>,
    pub text: String,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DoorDestination {
    pub room_name: String,
    pub dock_num: u32,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DoorConfig {
    #[serde(alias = "type")]
    pub shield_type: Option<String>,
    pub blast_shield_type: Option<String>,
    pub destination: Option<DoorDestination>, // Must be in same area. Ex: "destination":"Main Plaza"
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuitColors {
    pub power_deg: Option<i16>,
    pub varia_deg: Option<i16>,
    pub gravity_deg: Option<i16>,
    pub phazon_deg: Option<i16>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefaultGameOptions {
    pub screen_brightness: Option<u32>,
    pub screen_offset_x: Option<i32>,
    pub screen_offset_y: Option<i32>,
    pub screen_stretch: Option<i32>,
    pub sound_mode: Option<u32>,
    pub sfx_volume: Option<u32>,
    pub music_volume: Option<u32>,
    pub visor_opacity: Option<u32>,
    pub helmet_opacity: Option<u32>,
    pub hud_lag: Option<bool>,
    pub reverse_y_axis: Option<bool>,
    pub rumble: Option<bool>,
    pub swap_beam_controls: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaterConfig {
    pub id: Option<u32>,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    #[serde(alias = "type")]
    pub liquid_type: String,
    pub position: [f32; 3],
    pub scale: [f32; 3],
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub enum PlatformType {
    Metal,
    Snow,
    Block,
    HalfBlock,
    LongBlock,
    BombBox,
    Empty,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformConfig {
    pub id: Option<u32>,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub position: [f32; 3],
    pub rotation: Option<[f32; 3]>,
    pub alt_platform: Option<bool>, // deprecated
    #[serde(alias = "type")]
    pub platform_type: Option<PlatformType>,
    pub xray_only: Option<bool>,
    pub thermal_only: Option<bool>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub enum GenericTexture {
    #[serde(alias = "grass", alias = "GRASS")]
    Grass, // TXTR_BE288047
    #[serde(alias = "crater", alias = "CRATER")]
    Crater, // TXTR_8E899523
    #[serde(alias = "mine", alias = "MINE")]
    Mine, // TXTR_7D77CEE0
    #[serde(alias = "snow", alias = "SNOW")]
    Snow, // TXTR_2E6E5FC1
    #[serde(alias = "sandstone", alias = "SANDSTONE")]
    Sandstone, // TXTR_AA452C33
}

impl GenericTexture {
    pub fn txtr(self) -> ResId<res_id::TXTR> {
        let id = match self {
            GenericTexture::Grass => 0xBE288047,
            GenericTexture::Crater => 0x8E899523,
            GenericTexture::Mine => 0x7D77CEE0,
            GenericTexture::Snow => 0x2E6E5FC1,
            GenericTexture::Sandstone => 0xAA452C33,
        };

        ResId::new(id)
    }

    pub fn cmdl(self) -> ResId<res_id::CMDL> {
        match self {
            GenericTexture::Grass => custom_asset_ids::BLOCK_COLOR_0,
            GenericTexture::Crater => custom_asset_ids::BLOCK_COLOR_1,
            GenericTexture::Mine => custom_asset_ids::BLOCK_COLOR_2,
            GenericTexture::Snow => custom_asset_ids::BLOCK_COLOR_3,
            GenericTexture::Sandstone => custom_asset_ids::BLOCK_COLOR_4,
        }
    }

    pub fn dependencies(self) -> Vec<(u32, FourCC)> {
        vec![
            (self.cmdl().to_u32(), FourCC::from_bytes(b"CMDL")),
            (self.txtr().to_u32(), FourCC::from_bytes(b"TXTR")),
        ]
    }

    pub fn iter() -> impl Iterator<Item = GenericTexture> {
        [
            GenericTexture::Grass,
            GenericTexture::Crater,
            GenericTexture::Mine,
            GenericTexture::Snow,
            GenericTexture::Sandstone,
        ]
        .iter()
        .copied()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlockConfig {
    pub id: Option<u32>,
    pub active: Option<bool>,
    pub layer: Option<u32>,
    pub position: [f32; 3],
    pub scale: Option<[f32; 3]>,
    pub texture: Option<GenericTexture>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EscapeSequenceConfig {
    pub time: Option<f32>,
    pub start_trigger_pos: [f32; 3],
    pub start_trigger_scale: [f32; 3],
    pub stop_trigger_pos: [f32; 3],
    pub stop_trigger_scale: [f32; 3],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraHintConfig {
    pub layer: Option<u32>,
    pub trigger_id: Option<u32>,
    pub trigger_pos: [f32; 3],
    pub trigger_scale: [f32; 3],
    pub camera_id: Option<u32>,
    pub camera_pos: [f32; 3],
    pub camera_rot: [f32; 3],

    /**
       enum class EBallCameraBehaviour {
           Default,
           FreezeLookPosition, // Unused
           HintBallToCam,
           HintInitializePosition,
           HintFixedPosition,
           HintFixedTransform,
           PathCameraDesiredPos, // Unused
           PathCamera,
           SpindleCamera
       };
    */
    pub behavior: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockOnPoint {
    pub id1: Option<u32>,
    pub layer: Option<u32>,
    pub active1: Option<bool>,
    pub id2: Option<u32>,
    pub active2: Option<bool>,
    pub position: [f32; 3],
    pub is_grapple: Option<bool>,
    pub no_lock: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum DamageType {
    #[serde(alias = "power", alias = "POWER")]
    Power,
    #[serde(alias = "ice", alias = "ICE")]
    Ice,
    #[serde(alias = "wave", alias = "WAVE")]
    Wave,
    #[serde(alias = "plasma", alias = "PLASMA")]
    Plasma,
    #[serde(alias = "bomb", alias = "BOMB")]
    Bomb,
    #[serde(alias = "powerbomb", alias = "POWERBOMB")]
    PowerBomb,
    #[serde(alias = "missile", alias = "MISSILE")]
    Missile,
    #[serde(alias = "boostball", alias = "BOOSTBALL")]
    BoostBall,
    #[serde(alias = "phazon", alias = "PHAZON")]
    Phazon,
    #[serde(alias = "ai", alias = "AI")]
    Ai,
    #[serde(alias = "poisonwater", alias = "POISONWATER")]
    PoisonWater,
    #[serde(alias = "lava", alias = "LAVA")]
    Lava,
    #[serde(alias = "hot", alias = "HOT")]
    Hot,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerConfig {
    pub id: Option<u32>,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub position: Option<[f32; 3]>,
    pub scale: Option<[f32; 3]>,
    pub force: Option<[f32; 3]>,
    pub damage_type: Option<DamageType>,
    pub damage_amount: Option<f32>,
    pub flags: Option<u32>,
    pub deactivate_on_enter: Option<bool>,
    pub deactivate_on_exit: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum SpecialFunctionType {
    What = 0,
    #[serde(alias = "playerFollowLocator", alias = "PLAYERFOLLOWLOCATOR")]
    PlayerFollowLocator,
    #[serde(alias = "spinnerController", alias = "SPINNERCONTROLLER")]
    SpinnerController,
    #[serde(alias = "objectFollowLocator", alias = "OBJECTFOLLOWLOCATOR")]
    ObjectFollowLocator,
    #[serde(alias = "chaffTarget", alias = "CHAFFTARGET")]
    ChaffTarget,
    #[serde(alias = "inventoryActivator", alias = "INVENTORYACTIVATOR")]
    InventoryActivator,
    #[serde(alias = "mapStation", alias = "MAPSTATION")]
    MapStation,
    #[serde(alias = "saveStation", alias = "SAVESTATION")]
    SaveStation,
    #[serde(alias = "introBossRingController", alias = "INTROBOSSRINGCONTROLLER")]
    IntroBossRingController,
    #[serde(alias = "viewFrustumTest", alias = "VIEWFRUSTUMTEST")]
    ViewFrustumTest,
    #[serde(alias = "shotSpinnerController", alias = "SHOTSPINNERCONTROLLER")]
    ShotSpinnerController,
    #[serde(alias = "escapeSequence", alias = "ESCAPESEQUENCE")]
    EscapeSequence,
    #[serde(alias = "bossEnergyBar", alias = "BOSSENERGYBAR")]
    BossEnergyBar,
    #[serde(alias = "endGame", alias = "ENDGAME")]
    EndGame,
    #[serde(alias = "hUDFadeIn", alias = "HUDFADEIN")]
    HUDFadeIn,
    #[serde(alias = "cinematicSkip", alias = "CINEMATICSKIP")]
    CinematicSkip,
    #[serde(alias = "scriptLayerController", alias = "SCRIPTLAYERCONTROLLER")]
    ScriptLayerController,
    #[serde(alias = "rainSimulator", alias = "RAINSIMULATOR")]
    RainSimulator,
    #[serde(alias = "areaDamage", alias = "AREADAMAGE")]
    AreaDamage,
    #[serde(alias = "objectFollowObject", alias = "OBJECTFOLLOWOBJECT")]
    ObjectFollowObject,
    #[serde(alias = "hintSystem", alias = "HINTSYSTEM")]
    HintSystem,
    #[serde(alias = "dropBomb", alias = "DROPBOMB")]
    DropBomb,
    #[serde(alias = "scaleActor", alias = "SCALEACTOR")]
    ScaleActor,
    #[serde(alias = "missileStation", alias = "MISSILESTATION")]
    MissileStation,
    #[serde(alias = "billboard", alias = "BILLBOARD")]
    Billboard,
    #[serde(alias = "playerInAreaRelay", alias = "PLAYERINAREARELAY")]
    PlayerInAreaRelay,
    #[serde(alias = "hUDTarget", alias = "HUDTARGET")]
    HUDTarget,
    #[serde(alias = "fogFader", alias = "FOGFADER")]
    FogFader,
    #[serde(alias = "enterLogbook", alias = "ENTERLOGBOOK")]
    EnterLogbook,
    #[serde(alias = "powerBombStation", alias = "POWERBOMBSTATION")]
    PowerBombStation,
    #[serde(alias = "ending", alias = "ENDING")]
    Ending,
    #[serde(alias = "fusionRelay", alias = "FUSIONRELAY")]
    FusionRelay,
    #[serde(alias = "weaponSwitch", alias = "WEAPONSWITCH")]
    WeaponSwitch,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpecialFunctionConfig {
    pub id: Option<u32>,
    pub layer: Option<u32>,

    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,

    #[serde(alias = "type")]
    pub type_: SpecialFunctionType,

    pub unknown1: Option<String>,
    pub unknown2: Option<f32>,
    pub unknown3: Option<f32>,
    pub unknown4: Option<f32>,

    pub layer_change_room_id: Option<u32>,
    pub layer_change_layer_id: Option<u32>,
    pub item_id: Option<String>,

    pub active: Option<bool>,
    pub unknown6: Option<f32>,

    // "Used by SpinnerController"
    pub spinner1: Option<u32>,
    pub spinner2: Option<u32>,
    pub spinner3: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorRotateConfig {
    pub id: Option<u32>,
    pub layer: Option<u32>,
    pub rotation: [f32; 3],
    pub time_scale: f32,
    pub update_actors: bool,
    pub update_on_creation: bool,
    pub update_active: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StreamedAudioConfig {
    pub id: Option<u32>,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub audio_file_name: String,
    pub no_stop_on_deactivate: Option<bool>,
    pub fade_in_time: Option<f32>,
    pub fade_out_time: Option<f32>,
    pub volume: Option<u32>,
    pub oneshot: Option<u32>,
    pub is_music: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditObjConfig {
    pub layer: Option<u32>,
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
    pub scale: Option<[f32; 3]>,
    pub size: Option<f32>,
    pub speed: Option<f32>,
    pub damage: Option<f32>,
    pub detection_range: Option<f32>,
    pub attack_range: Option<f32>,
    pub vulnerability: Option<String>, // maps to DoorType
    pub vulnerabilities: Option<HashMap<u32, String>>,
    pub health: Option<f32>,
    pub healths: Option<HashMap<u32, f32>>,
}

// None = 0,
// PerspLin = 2,
// PerspExp = 4,
// PerspExp2 = 5,
// PerspRevExp = 6,
// PerspRevExp2 = 7,
// OrthoLin = 10,
// OrthoExp = 12,
// OrthoExp2 = 13,
// OrthoRevExp = 14,
// OrthoRevExp2 = 15,

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FogConfig {
    pub id: Option<u32>,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub mode: Option<u32>,
    pub explicit: Option<bool>,
    pub color: Option<[f32; 4]>, // RGBA
    pub range: Option<[f32; 2]>, // X, Y
    pub color_delta: Option<f32>,
    pub range_delta: Option<[f32; 2]>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BombSlotConfig {
    pub damageable_trigger_id: u32,
    pub platform_id: Option<u32>,
    pub actor_id: Option<u32>,
    pub ball_trigger_id: Option<u32>,
    pub activate_slot_id: Option<u32>,
    pub deactivate_slot_id: Option<u32>,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub release_ball_delay_s: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerActorConfig {
    pub id: Option<u32>,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositionConfig {
    pub trigger_position: [f32; 3],
    pub trigger_scale: [f32; 3],
    pub destination_position: [f32; 3],
    pub destination_rotation: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HudmemoConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub text: Option<String>,
    pub message_time: Option<f32>,
    pub modal: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaypointConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
    pub active: Option<bool>,
    pub speed: Option<f32>,
    pub pause: Option<f32>,
    pub pattern_translate: Option<u32>,
    pub pattern_orient: Option<u32>,
    pub pattern_fit: Option<u32>,
    pub behaviour: Option<u32>,
    pub behaviour_orient: Option<u32>,
    pub behaviour_modifiers: Option<u32>,
    pub animation: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CounterConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub start_value: Option<u32>,
    pub max_value: Option<u32>,
    pub auto_reset: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwitchConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub open: Option<bool>,
    pub auto_close: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerHintConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub priority: Option<u32>,
    pub unknown1: Option<bool>,
    pub unknown2: Option<bool>,
    pub extend_target_distance: Option<bool>,
    pub unknown4: Option<bool>,
    pub unknown5: Option<bool>,
    pub disable_unmorph: Option<bool>,
    pub disable_morph: Option<bool>,
    pub disable_controls: Option<bool>,
    pub disable_boost: Option<bool>,
    pub activate_visor_combat: Option<bool>,
    pub activate_visor_scan: Option<bool>,
    pub activate_visor_thermal: Option<bool>,
    pub activate_visor_xray: Option<bool>,
    pub unknown6: Option<bool>,
    pub face_object_on_unmorph: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum EnviornmentalEffect {
    None,
    Snow,
    Rain,
    Bubbles,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ControllerActionType {
    Forward = 0,
    Backward,
    TurnLeft,
    TurnRight,
    StrafeLeft,
    StrafeRight,
    LookLeft,
    LookRight,
    LookUp,
    LookDown,
    JumpOrBoost,
    FireOrBomb,
    MissileOrPowerBomb,
    Morph,
    AimUp,
    AimDown,
    CycleBeamUp,
    CycleBeamDown,
    CycleItem,
    PowerBeam,
    IceBeam,
    WaveBeam,
    PlasmaBeam,
    ToggleHolster,
    OrbitClose,
    OrbitFar,
    OrbitObject,
    OrbitSelect,
    OrbitConfirm,
    OrbitLeft,
    OrbitRight,
    OrbitUp,
    OrbitDown,
    LookHold1,
    LookHold2,
    LookZoomIn,
    LookZoomOut,
    AimHold,
    MapCircleUp,
    MapCircleDown,
    MapCircleLeft,
    MapCircleRight,
    MapMoveForward,
    MapMoveBack,
    MapMoveLeft,
    MapMoveRight,
    MapZoomIn,
    MapZoomOut,
    SpiderBall,
    ChaseCamera,
    XRayVisor,
    ThermoVisor,
    EnviroVisor,
    NoVisor,
    VisorMenu,
    VisorUp,
    VisorDown,
    Unknown1,
    Unknown2,
    UseShield,
    ScanItem,
    Unknown3,
    Unknown4,
    Unknown5,
    Unknown6,
    PreviousPauseScreen,
    NextPauseScreen,
    Unknown7,
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ControllerActionConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub active: Option<bool>,

    pub action: ControllerActionType,

    pub one_shot: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
    pub active: Option<bool>,
    pub shot_duration: Option<f32>,
    pub look_at_player: Option<bool>,
    pub out_of_player_eye: Option<bool>,
    pub into_player_eye: Option<bool>,
    pub draw_player: Option<bool>,
    pub disable_input: Option<bool>,
    pub unknown: Option<bool>,
    pub finish_cine_skip: Option<bool>,
    pub field_of_view: Option<f32>,
    pub check_failsafe: Option<bool>,
    pub disable_out_of_into: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraWaypointConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
    pub active: Option<bool>,
    pub fov: Option<f32>,
    pub unknown: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum FilterType {
    Passthrough = 0,
    Multiply,
    Invert,
    Add,
    Subtract,
    Blend,
    Widescreen,
    SceneAdd,
    NoColor,
    InvDstMultiply,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum FilterShape {
    Fullscreen = 0,
    FullscreenHalvesLeftRight,
    FullscreenHalvesTopBottom,
    FullscreenQuarters,
    CinemaBars,
    ScanLinesEven,
    ScanLinesOdd,
    RandomStatic,
    CookieCutterDepthRandomStatic,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraFilterKeyframeConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub active: Option<bool>,

    pub filter_type: FilterType,
    pub filter_shape: FilterShape,

    pub filter_index: Option<u32>,
    pub filter_group: Option<u32>,
    pub fade_in_time: Option<f32>,
    pub fade_out_time: Option<f32>,
    pub color: Option<[f32; 4]>,
    pub overlay_texture: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum EBallCameraBehaviour {
    Default,
    FreezeLookPosition, // Unused
    HintBallToCam,
    HintInitializePosition,
    HintFixedPosition,
    HintFixedTransform,
    PathCameraDesiredPos, // Unused
    PathCamera,
    SpindleCamera,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewCameraHintConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
    pub active: Option<bool>,
    pub priority: Option<u32>,
    pub behaviour: EBallCameraBehaviour,
    pub calculate_cam_pos: Option<bool>,
    pub chase_allowed: Option<bool>,
    pub boost_allowed: Option<bool>,
    pub obscure_avoidance: Option<bool>,
    pub volume_collider: Option<bool>,
    pub apply_immediately: Option<bool>,
    pub look_at_ball: Option<bool>,
    pub hint_distance_selection: Option<bool>,
    pub hint_distance_self_pos: Option<bool>,
    pub control_interpolation: Option<bool>,
    pub sinusoidal_interpolation: Option<bool>,
    pub sinusoidal_interpolation_hintless: Option<bool>,
    pub clamp_velocity: Option<bool>,
    pub skip_cinematic: Option<bool>,
    pub no_elevation_interp: Option<bool>,
    pub direct_elevation: Option<bool>,
    pub override_look_dir: Option<bool>,
    pub no_elevation_vel_clamp: Option<bool>,
    pub calculate_transform_from_prev_cam: Option<bool>,
    pub no_spline: Option<bool>,
    pub unknown1: Option<bool>,
    pub unknown2: Option<bool>,
    pub override_min_dist: Option<bool>,
    pub min_dist: Option<f32>,
    pub override_max_dist: Option<bool>,
    pub max_dist: Option<f32>,
    pub override_backwards_dist: Option<bool>,
    pub backwards_dist: Option<f32>,
    pub override_look_at_offset: Option<bool>,
    pub look_at_offset: Option<[f32; 3]>,
    pub override_chase_look_at_offset: Option<bool>,
    pub chase_look_at_offset: Option<[f32; 3]>,
    pub ball_to_cam: Option<[f32; 3]>,
    pub override_fov: Option<bool>,
    pub fov: Option<f32>,
    pub override_attitude_range: Option<bool>,
    pub attitude_range: Option<f32>,
    pub override_azimuth_range: Option<bool>,
    pub azimuth_range: Option<f32>,
    pub override_angle_per_second: Option<bool>,
    pub angle_per_second: Option<f32>,
    pub clamp_vel_range: Option<f32>,
    pub clamp_rot_range: Option<f32>,
    pub override_elevation: Option<bool>,
    pub elevation: Option<f32>,
    pub interpolate_time: Option<f32>,
    pub clamp_vel_time: Option<f32>,
    pub control_interp_dur: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraHintTriggerConfig {
    pub id: Option<u32>,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
    pub scale: Option<[f32; 3]>,
    pub deactivate_on_enter: Option<bool>,
    pub deactivate_on_exit: Option<bool>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum ConnectionState {
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
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum ConnectionMsg {
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionConfig {
    pub sender_id: u32,
    pub target_id: u32,
    pub state: ConnectionState,
    pub message: ConnectionMsg,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelayConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub active: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimerConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub time: f32,
    pub max_random_add: Option<f32>,
    pub looping: Option<bool>,
    pub start_immediately: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorKeyFrameConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub animation_id: u32,
    pub looping: bool,
    pub lifetime: f32,
    pub fade_out: f32,
    pub total_playback: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpawnPointConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub position: [f32; 3],
    pub rotation: Option<[f32; 3]>,
    pub default_spawn: Option<bool>,
    pub morphed: Option<bool>,
    pub items: Option<StartingItems>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldLightFaderConfig {
    pub id: u32,
    pub layer: Option<u32>,
    pub active: Option<bool>,
    pub faded_light_level: Option<f32>,
    pub fade_speed: Option<f32>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoomConfig {
    pub superheated: Option<bool>,
    pub remove_water: Option<bool>,
    pub submerge: Option<bool>,
    pub map_default_state: Option<MapaObjectVisibilityMode>,
    pub liquids: Option<Vec<WaterConfig>>,
    pub pickups: Option<Vec<PickupConfig>>,
    pub extra_scans: Option<Vec<ScanConfig>>,
    pub doors: Option<HashMap<u32, DoorConfig>>,
    pub spawn_position_override: Option<[f32; 3]>,
    pub bounding_box_offset: Option<[f32; 3]>,
    pub bounding_box_scale: Option<[f32; 3]>,
    pub platforms: Option<Vec<PlatformConfig>>,
    pub camera_hints: Option<Vec<CameraHintConfig>>,
    pub blocks: Option<Vec<BlockConfig>>,
    pub lock_on_points: Option<Vec<LockOnPoint>>,
    pub fog: Option<FogConfig>,
    pub ambient_lighting_scale: Option<f32>, // 1.0 is default lighting
    pub enviornmental_effect: Option<EnviornmentalEffect>,
    pub initial_enviornmental_effect: Option<f32>,
    pub initial_thermal_heat_level: Option<f32>,
    pub xray_fog_distance: Option<f32>,
    pub escape_sequences: Option<Vec<EscapeSequenceConfig>>,
    pub repositions: Option<Vec<RepositionConfig>>,
    pub hudmemos: Option<Vec<HudmemoConfig>>,
    pub layers: Option<HashMap<u32, bool>>,
    pub layer_objs: Option<HashMap<u32, u32>>,
    pub delete_ids: Option<Vec<u32>>,
    pub audio_override: Option<HashMap<String, String>>, // key=instance_id, value=/audio/min_phazonL.dsp|/audio/min_phazonR.dsp
    pub add_connections: Option<Vec<ConnectionConfig>>,
    pub remove_connections: Option<Vec<ConnectionConfig>>,
    pub relays: Option<Vec<RelayConfig>>,
    pub cutscene_skip_fns: Option<Vec<u32>>, // instance id of new special function
    pub timers: Option<Vec<TimerConfig>>,
    pub actor_keyframes: Option<Vec<ActorKeyFrameConfig>>,
    pub spawn_points: Option<Vec<SpawnPointConfig>>,
    pub triggers: Option<Vec<TriggerConfig>>,
    pub special_functions: Option<Vec<SpecialFunctionConfig>>,
    pub actor_rotates: Option<Vec<ActorRotateConfig>>,
    pub streamed_audios: Option<Vec<StreamedAudioConfig>>,
    pub edit_objs: Option<HashMap<u32, EditObjConfig>>,
    pub waypoints: Option<Vec<WaypointConfig>>,
    pub counters: Option<Vec<CounterConfig>>,
    pub switches: Option<Vec<SwitchConfig>>,
    pub player_hints: Option<Vec<PlayerHintConfig>>,
    pub distance_fogs: Option<Vec<FogConfig>>,
    pub bomb_slots: Option<Vec<BombSlotConfig>>,
    pub controller_actions: Option<Vec<ControllerActionConfig>>,
    pub player_actors: Option<Vec<PlayerActorConfig>>,
    pub world_light_faders: Option<Vec<WorldLightFaderConfig>>,
    pub cameras: Option<Vec<CameraConfig>>,
    pub camera_waypoints: Option<Vec<CameraWaypointConfig>>,
    pub camera_filter_keyframes: Option<Vec<CameraFilterKeyframeConfig>>,
    pub new_camera_hints: Option<Vec<NewCameraHintConfig>>,
    pub camera_hint_triggers: Option<Vec<CameraHintTriggerConfig>>,
    // Don't forget to update merge_json when adding here
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LevelConfig {
    #[serde(default)]
    pub transports: HashMap<String, String>,

    #[serde(default)]
    pub rooms: HashMap<String, RoomConfig>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CtwkConfig {
    pub fov: Option<f32>,
    pub player_size: Option<f32>,
    pub morph_ball_size: Option<f32>,
    pub easy_lava_escape: Option<bool>,
    pub move_while_scan: Option<bool>,
    pub scan_range: Option<f32>,
    pub bomb_jump_height: Option<f32>,
    pub bomb_jump_radius: Option<f32>,
    pub grapple_beam_speed: Option<f32>,
    pub aim_assist_angle: Option<f32>,
    pub gravity: Option<f32>,
    pub ice_break_timeout: Option<f32>,
    pub ice_break_jump_count: Option<u32>,
    pub ground_friction: Option<f32>,
    pub coyote_frames: Option<u32>,
    pub move_during_free_look: Option<bool>,
    pub recenter_after_freelook: Option<bool>,
    pub max_speed: Option<f32>,
    pub max_acceleration: Option<f32>,
    pub space_jump_impulse: Option<f32>,
    pub vertical_space_jump_accel: Option<f32>,
    pub horizontal_space_jump_accel: Option<f32>,
    pub eye_offset: Option<f32>,
    pub toggle_free_look: Option<bool>,
    pub two_buttons_for_free_look: Option<bool>,
    pub disable_dash: Option<bool>,
    pub varia_damage_reduction: Option<f32>,
    pub gravity_damage_reduction: Option<f32>,
    pub phazon_damage_reduction: Option<f32>,
    pub hardmode_damage_mult: Option<f32>,
    pub hardmode_weapon_mult: Option<f32>,
    pub turn_speed: Option<f32>,
    pub underwater_fog_distance: Option<f32>,
    pub step_up_height: Option<f32>,
    pub allowed_jump_time: Option<f32>,
    pub allowed_space_jump_time: Option<f32>,
    pub min_space_jump_window: Option<f32>,
    pub max_space_jump_window: Option<f32>,
    pub min_jump_time: Option<f32>,
    pub min_space_jump_time: Option<f32>,
    pub falling_space_jump: Option<bool>,
    pub impulse_space_jump: Option<bool>,

    // PlayerGun.CTWK
    pub gun_position: Option<[f32; 3]>, // offset
    pub gun_damage: Option<f32>,
    pub gun_cooldown: Option<f32>,

    // Ball.CTWK
    pub max_translation_accel: Option<f32>,
    pub translation_friction: Option<f32>,
    pub translation_max_speed: Option<f32>,
    pub ball_forward_braking_accel: Option<f32>,
    pub ball_gravity: Option<f32>,
    pub ball_water_gravity: Option<f32>,
    pub boost_drain_time: Option<f32>,
    pub boost_min_charge_time: Option<f32>,
    pub boost_min_rel_speed_for_damage: Option<f32>,
    pub boost_charge_time0: Option<f32>,
    pub boost_charge_time1: Option<f32>,
    pub boost_charge_time2: Option<f32>,
    pub boost_incremental_speed0: Option<f32>,
    pub boost_incremental_speed1: Option<f32>,
    pub boost_incremental_speed2: Option<f32>,

    // GuiColors.CTWK
    pub hud_color: Option<[f32; 3]>, // RGB, 0 - 1.0
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IncineratorDroneConfig {
    pub contraption_start_delay_minimum_time: Option<f32>,
    pub contraption_start_delay_random_time: Option<f32>,
    pub eye_stay_up_minimum_time: Option<f32>,
    pub eye_stay_up_random_time: Option<f32>,
    pub eye_wait_initial_minimum_time: Option<f32>,
    pub eye_wait_initial_random_time: Option<f32>,
    pub eye_wait_minimum_time: Option<f32>,
    pub eye_wait_random_time: Option<f32>,
    pub reset_contraption_minimum_time: Option<f32>,
    pub reset_contraption_random_time: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HallOfTheEldersBombSlotCoversConfig {
    pub wave: Option<BombSlotCover>,
    pub ice: Option<BombSlotCover>,
    pub plasma: Option<BombSlotCover>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum BombSlotCover {
    #[serde(alias = "wave", alias = "WAVE")]
    Wave,
    #[serde(alias = "ice", alias = "ICE")]
    Ice,
    #[serde(alias = "plasma", alias = "PLASMA")]
    Plasma,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub enum PhazonDamageModifier {
    Default,
    LinearDelayed, // Default but the damages don't increase over time
    Linear,        // Starts directly and deals linear damages
}

#[derive(Serialize, Debug, PartialEq, Copy, Clone)]
pub enum Version {
    NtscU0_00,
    NtscU0_01,
    NtscU0_02,
    NtscK,
    NtscJ,
    Pal,
    NtscUTrilogy,
    NtscJTrilogy,
    PalTrilogy,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Copy, Clone)]
pub enum DoorOpenMode {
    Original,           // Doors always stay the same color after opening
    PrimaryBlastShield, // Doors under blast shields downgrade their vulnerability to Blue/Wave/Ice/Plasma from vulnerabilities with more requirements after opening
    // PrimaryAll, // All doors downgrade their vulnerability to Blue/Wave/Ice/Plasma from vulnerabilities with more requirements after opening
    BlueBlastShield, // Doors under blast shields downgrade to Blue doors after opening
                     // BlueAll, // All Doors downgrade to Blue after opening
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Copy, Clone)]
pub enum DifficultyBehavior {
    Either,
    NormalOnly,
    HardOnly,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub enum SuitDamageReduction {
    #[default]
    Default,
    Progressive,
    Additive,
}

impl<'de> Deserialize<'de> for SuitDamageReduction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SuitDamageReductionVisitor;

        impl Visitor<'_> for SuitDamageReductionVisitor {
            type Value = SuitDamageReduction;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    r#""Default", "Progressive", "Additive", true, or false"#
                )
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if v {
                    Ok(SuitDamageReduction::Progressive)
                } else {
                    Ok(SuitDamageReduction::Default)
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match v {
                    "Default" => Ok(SuitDamageReduction::Default),
                    "Progressive" => Ok(SuitDamageReduction::Progressive),
                    "Additive" => Ok(SuitDamageReduction::Additive),
                    variant => Err(E::unknown_variant(
                        variant,
                        &["Default", "Progressive", "Additive"],
                    )),
                }
            }
        }

        deserializer.deserialize_any(SuitDamageReductionVisitor)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Version::NtscU0_00 => write!(f, "1.00"),
            Version::NtscU0_01 => write!(f, "1.01"),
            Version::NtscU0_02 => write!(f, "1.02"),
            Version::NtscK => write!(f, "kor"),
            Version::NtscJ => write!(f, "jpn"),
            Version::Pal => write!(f, "pal"),
            Version::NtscUTrilogy => write!(f, "trilogy_ntsc_u"),
            Version::NtscJTrilogy => write!(f, "trilogy_ntsc_j"),
            Version::PalTrilogy => write!(f, "trilogy_pal"),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PatchConfig {
    pub run_mode: RunMode,
    pub logbook_filename: Option<String>,
    pub export_asset_dir: Option<String>,
    pub extern_assets_dir: Option<String>,
    pub seed: u64,
    pub uuid: Option<[u8; 16]>,

    pub force_vanilla_layout: bool,

    pub version: Version,

    #[serde(skip_serializing)]
    pub input_iso: memmap::Mmap,
    pub iso_format: IsoFormat,
    #[serde(skip_serializing)]
    pub output_iso: File,

    pub qol_cutscenes: CutsceneMode,
    pub qol_game_breaking: bool,
    pub qol_cosmetic: bool,
    pub qol_pickup_scans: bool,
    pub qol_general: bool,

    pub phazon_elite_without_dynamo: bool,
    pub main_plaza_door: bool,
    pub backwards_labs: bool,
    pub backwards_frigate: bool,
    pub backwards_upper_mines: bool,
    pub backwards_lower_mines: bool,
    pub patch_power_conduits: bool,
    pub remove_mine_security_station_locks: bool,
    pub remove_hive_mecha: bool,
    pub power_bomb_arboretum_sandstone: bool,
    pub visible_bounding_box: bool,
    pub door_destination_scans: bool,
    pub no_hud: bool,

    pub incinerator_drone_config: Option<IncineratorDroneConfig>,
    pub hall_of_the_elders_bomb_slot_covers: Option<HallOfTheEldersBombSlotCoversConfig>,
    pub maze_seeds: Option<Vec<u32>>,

    #[serde(skip_serializing)] // stop racers from peeking at locations
    pub level_data: HashMap<String, LevelConfig>,

    pub strg: HashMap<String, Vec<String>>, // "<decimal asset ID>": <non-null terminated table of strings>

    pub starting_room: String,
    pub starting_memo: Option<String>,
    pub spring_ball: bool,
    pub spring_ball_item: PickupType,
    pub warp_to_start: bool,
    pub warp_to_start_delay_s: f32,

    pub automatic_crash_screen: bool,
    pub etank_capacity: u32,
    pub shuffle_pickup_position: bool,
    pub shuffle_pickup_pos_all_rooms: bool,
    pub remove_vanilla_blast_shields: bool,
    pub nonvaria_heat_damage: bool,
    pub heat_damage_per_sec: f32,
    pub poison_damage_per_sec: f32,
    pub phazon_damage_per_sec: f32,
    pub phazon_damage_modifier: PhazonDamageModifier,
    pub staggered_suit_damage: SuitDamageReduction,
    pub item_max_capacity: HashMap<PickupType, u32>,
    pub map_default_state: MapaObjectVisibilityMode,
    pub auto_enabled_elevators: bool,
    pub skip_ridley: bool,
    pub multiworld_dol_patches: bool,
    pub update_hint_state_replacement: Option<Vec<u8>>,
    pub quiet: bool,

    pub starting_items: StartingItems,
    pub item_loss_items: StartingItems,
    pub disable_item_loss: bool,
    pub starting_visor: Visor,
    pub starting_beam: Beam,
    pub escape_sequence_counts_up: bool,
    pub enable_ice_traps: bool, // deprecated
    pub missile_station_pb_refill: bool,
    pub door_open_mode: DoorOpenMode,

    pub artifact_hint_behavior: ArtifactHintBehavior,

    #[serde(skip_serializing)]
    pub flaahgra_music_files: Option<[Vec<u8>; 2]>,

    pub skip_splash_screens: bool,
    pub default_game_options: Option<DefaultGameOptions>,
    pub suit_colors: Option<SuitColors>,
    pub force_fusion: bool,
    pub cache_dir: String,

    pub quickplay: bool,
    pub quickpatch: bool,

    pub game_banner: GameBanner,
    pub comment: String,
    pub main_menu_message: String,

    #[serde(skip_serializing)] // skipped for competitive integrity reasons
    pub credits_string: Option<String>,

    pub results_string: Option<String>,
    pub artifact_hints: Option<HashMap<String, String>>, // e.g. "Strength":"This item can be found in Ruined Fountain"
    pub required_artifact_count: Option<u32>,
    pub artifact_temple_layer_overrides: Option<HashMap<String, bool>>,
    pub no_doors: bool,
    pub boss_sizes: HashMap<String, f32>,
    pub shoot_in_grapple: bool,
    pub difficulty_behavior: DifficultyBehavior,
    pub legacy_block_size: bool,
    pub patch_wallcrawling: bool,
    pub blast_shield_lockon: bool,
    pub ctwk_config: CtwkConfig,
}

/*** Un-Parsed Config (doubles as JSON input specification) ***/

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Preferences {
    skip_splash_screens: Option<bool>,
    default_game_options: Option<DefaultGameOptions>,
    suit_colors: Option<SuitColors>,
    force_fusion: Option<bool>,
    cache_dir: Option<String>,

    qol_game_breaking: Option<bool>,
    qol_cosmetic: Option<bool>,
    qol_cutscenes: Option<String>,
    qol_pickup_scans: Option<bool>,
    qol_general: Option<bool>,

    map_default_state: Option<String>,
    artifact_hint_behavior: Option<String>,
    automatic_crash_screen: Option<bool>,
    visible_bounding_box: Option<bool>,
    door_destination_scans: Option<bool>,
    no_hud: Option<bool>,

    trilogy_disc_path: Option<String>,
    quickplay: Option<bool>,
    quickpatch: Option<bool>,
    quiet: Option<bool>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GameConfig {
    starting_room: Option<String>,
    starting_memo: Option<String>,
    spring_ball: Option<bool>,
    spring_ball_item: Option<String>,
    warp_to_start: Option<bool>,
    warp_to_start_delay_s: Option<f32>,

    shuffle_pickup_position: Option<bool>,
    shuffle_pickup_pos_all_rooms: Option<bool>,
    remove_vanilla_blast_shields: Option<bool>,
    nonvaria_heat_damage: Option<bool>,
    staggered_suit_damage: Option<SuitDamageReduction>,
    heat_damage_per_sec: Option<f32>,
    poison_damage_per_sec: Option<f32>,
    phazon_damage_per_sec: Option<f32>,
    phazon_damage_modifier: Option<String>,
    auto_enabled_elevators: Option<bool>,
    skip_ridley: Option<bool>,
    multiworld_dol_patches: Option<bool>,
    update_hint_state_replacement: Option<Vec<u8>>,

    starting_items: Option<StartingItems>,
    item_loss_items: Option<StartingItems>,
    disable_item_loss: Option<bool>,
    starting_visor: Option<String>,
    starting_beam: Option<String>,
    escape_sequence_counts_up: Option<bool>,
    enable_ice_traps: Option<bool>,
    missile_station_pb_refill: Option<bool>,
    door_open_mode: Option<DoorOpenMode>,

    etank_capacity: Option<u32>,
    item_max_capacity: Option<HashMap<String, u32>>,

    phazon_elite_without_dynamo: Option<bool>,
    main_plaza_door: Option<bool>,
    backwards_labs: Option<bool>,
    backwards_frigate: Option<bool>,
    backwards_upper_mines: Option<bool>,
    backwards_lower_mines: Option<bool>,
    patch_power_conduits: Option<bool>,
    remove_mine_security_station_locks: Option<bool>,
    remove_hive_mecha: Option<bool>,
    power_bomb_arboretum_sandstone: Option<bool>,

    incinerator_drone_config: Option<IncineratorDroneConfig>,
    maze_seeds: Option<Vec<u32>>,
    hall_of_the_elders_bomb_slot_covers: Option<HallOfTheEldersBombSlotCoversConfig>,

    game_banner: Option<GameBanner>,
    comment: Option<String>,
    main_menu_message: Option<String>,

    credits_string: Option<String>,
    results_string: Option<String>,
    artifact_hints: Option<HashMap<String, String>>, // e.g. "Strength":"This item can be found in Ruined Fountain"
    artifact_temple_layer_overrides: Option<HashMap<String, bool>>,
    required_artifact_count: Option<u32>,
    no_doors: Option<bool>, // Remove every door from the game
    boss_sizes: Option<HashMap<String, f32>>,
    shoot_in_grapple: Option<bool>,
    difficulty_behavior: Option<DifficultyBehavior>,
    legacy_block_size: Option<bool>,
    patch_wallcrawling: Option<bool>,
    blast_shield_lockon: Option<bool>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PatchConfigPrivate {
    #[serde(alias = "$schema")]
    _schema: Option<String>,

    run_mode: Option<String>,
    logbook_filename: Option<String>,
    export_asset_dir: Option<String>,
    input_iso: Option<String>,
    output_iso: Option<String>,
    force_vanilla_layout: Option<bool>,
    extern_assets_dir: Option<String>,
    seed: Option<u64>,
    uuid: Option<[u8; 16]>,

    #[serde(default)]
    preferences: Preferences,

    #[serde(default)]
    game_config: GameConfig,

    #[serde(default)]
    tweaks: CtwkConfig,

    #[serde(default)]
    level_data: HashMap<String, LevelConfig>,

    #[serde(default)]
    strg: HashMap<String, Vec<String>>, // "<decimal asset ID>": <non-null terminated table of strings>
}

/*** Parse Patcher Input ***/

fn extend_option_vec<T>(dest: &mut Option<Vec<T>>, src: Option<Vec<T>>) {
    if let Some(src_vec) = src {
        if dest.is_none() {
            let dest_vec: Vec<T> = Vec::new();
            *dest = Some(dest_vec);
        }

        if let Some(dest_vec) = dest {
            dest_vec.extend(src_vec);
        };
    }
}

macro_rules! extend_option_vec {
    ($label:ident, $self:expr, $other:expr) => {
        extend_option_vec(&mut $self.$label, $other.$label.clone());
    };
}

macro_rules! merge_optional {
    ($label:ident, $self:expr, $other:expr, $room_name:expr) => {
        if let Some(other_value) = $other.$label {
            match $self.$label {
                Some(self_value) => {
                    if self_value != other_value {
                        panic!("Conflict in {}'s editObjs", $room_name);
                    }
                }
                None => {
                    $self.$label = Some(other_value);
                }
            }
        }
    };
}

impl PatchConfig {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let result = strip_jsonc_comments(json, true);
        let result = serde_json::from_str(result.as_str());
        let result: PatchConfigPrivate = result.map_err(|e| format!("JSON parse failed: {}", e))?;
        result.parse()
    }

    pub fn from_cli_options() -> Result<Self, String> {
        let matches = App::new("randomprime ISO patcher")
            .version(crate_version!())
            .arg(Arg::with_name("input iso path")
                .long("input-iso")
                .takes_value(true))
            .arg(Arg::with_name("output iso path")
                .long("output-iso")
                .takes_value(true))
            .arg(Arg::with_name("extern assets dir")
                .long("extern-assets-dir")
                .takes_value(true))
            .arg(Arg::with_name("profile json path")
                .long("profile")
                .help("Path to JSON file with patch configuration (cli config takes priority). See documentation for details.")
                .takes_value(true))
            .arg(Arg::with_name("force vanilla layout")
                .long("force-vanilla-layout")
                .help("use this to play the vanilla game, but with a custom size factor"))
            .arg(Arg::with_name("qol game breaking")
                .long("qol-game-breaking")
                .help("Fix soft locks and crashes that retro didn't bother addressing"))
            .arg(Arg::with_name("qol cosmetic")
                .long("qol-cosmetic")
                .help("Patch cutscenes to fix continuity errors and UI to improve QoL without affecting IGT or the story"))
            .arg(Arg::with_name("qol cutscenes")
                .long("qol-cutscenes")
                .help("Original, Competitive, Minor, Major")
                .takes_value(true))
            .arg(Arg::with_name("starting room")
                .long("starting-room")
                .help("Room which the player starts their adventure from. Format - <world>:<room name>, where <world> is [Frigate|Tallon|Chozo|Magmoor|Phendrana|Mines|Crater]")
                .takes_value(true))
            .arg(Arg::with_name("starting memo")
                .long("starting-memo")
                .help("String which is shown to the player after they start a new save file")
                .takes_value(true))
            .arg(Arg::with_name("spring ball")
                .long("spring-ball")
                .help("Allows player to use spring ball when bombs are acquired if not set"))
            .arg(Arg::with_name("spring ball item")
                .long("spring-ball-item")
                .help("Spring ball checks for this item when enabled"))
            .arg(Arg::with_name("warp to start")
                .long("warp-to-start")
                .help("Allows player to warp to start from any save station"))
            .arg(Arg::with_name("automatic crash screen")
                .long("automatic-crash-screen")
                .help("Makes the crash screen appear without any button combination required"))
            .arg(Arg::with_name("etank capacity")
                .long("etank-capacity")
                .help("Set the etank capacity and base health")
                .takes_value(true))
            .arg(Arg::with_name("nonvaria heat damage")
                .long("nonvaria-heat-damage")
                .help("If the Varia Suit has not been collect, heat damage applies"))
            .arg(Arg::with_name("heat damage per sec")
                .long("heat-damage-per-sec")
                .help("Set the heat damage per seconds spent in a superheated room")
                .takes_value(true))
            .arg(Arg::with_name("poison damage per sec")
                .long("poison-damage-per-sec")
                .help("Set the poison damage per seconds spent in poison water")
                .takes_value(true))
            .arg(Arg::with_name("phazon damage per sec")
                .long("phazon-damage-per-sec")
                .help("Set the phazon damage per seconds spent in phazon (Applies only when using linear damages)")
                .takes_value(true))
            .arg(Arg::with_name("phazon damage modifier")
                .long("phazon-damage-modifier")
                .help("Change the phazon damage modifier (Either default, linear or linear_delayed)")
                .takes_value(true))
            .arg(Arg::with_name("staggered suit damage")
                .long("staggered-suit-damage")
                .help(concat!("The suit damage reduction is determinted by the number of suits ",
                                "collected rather than the most powerful one collected.")))
            .arg(Arg::with_name("map default state")
                .long("map-default-state")
                .help("Change the default state of map for each world (Either default, visible or visited)")
                .takes_value(true))
            .arg(Arg::with_name("skip ridley")
                .long("skip-ridley")
                .help("Skips Ridley fight when you have all the required artifacts and activate the portal to impact crater"))
            .arg(Arg::with_name("auto enabled elevators")
                .long("auto-enabled-elevators")
                .help("Every elevator will be automatically enabled without scaning its terminal"))
            .arg(Arg::with_name("artifact hint behavior")
                .long("artifact-hint-behavior")
                .help("Set the behavior of artifact temple hints. Can be 'all', 'none', or 'default' (vanilla)")
                .takes_value(true))
            .arg(Arg::with_name("trilogy disc path")
                .long("flaahgra-music-disc-path")
                .help(concat!("Location of a ISO of Metroid Prime Trilogy. If provided the ",
                                "Flaahgra fight music will be used to replace the original"))
                .takes_value(true))
            .arg(Arg::with_name("quiet")
                .long("quiet")
                .help("Don't print the progress messages"))
            .arg(Arg::with_name("main menu message")
                .long("main-menu-message")
                .hidden(true)
                .takes_value(true))
            .arg(Arg::with_name("starting items")
                .long("starting-items")
                .takes_value(true)
                .validator(|s| s.parse::<u64>().map(|_| ())
                                            .map_err(|_| "Expected an integer".to_string())))
            .arg(Arg::with_name("item loss items")
                .long("item-loss-items")
                .takes_value(true)
                .validator(|s| s.parse::<u64>().map(|_| ())
                                            .map_err(|_| "Expected an integer".to_string())))
            .arg(Arg::with_name("quickplay")
                .long("quickplay")
                .hidden(true))
            .arg(Arg::with_name("skip_splash_screens")
                .long("skip_splash_screens")
                .hidden(true))
            .arg(Arg::with_name("quickpatch")
                .long("quickpatch")
                .hidden(true))
            .arg(Arg::with_name("text file comment")
                .long("text-file-comment")
                .hidden(true)
                .takes_value(true))
            .arg(Arg::with_name("run mode")
                .long("run-mode")
                .hidden(false)
                .takes_value(true))
            .get_matches();

        let mut patch_config = if matches.is_present("profile json path") {
            let json_path = matches.value_of("profile json path").unwrap();
            let cli_json_config_raw: &str = &fs::read_to_string(json_path)
                .map_err(|e| format!("Could not read JSON file: {}", e))
                .unwrap();

            serde_json::from_str(strip_jsonc_comments(cli_json_config_raw, true).as_str())
                .map_err(|e| format!("JSON parse failed: {}", e))?
        } else {
            PatchConfigPrivate::default()
        };

        macro_rules! populate_config_bool {
            ($matches:expr; $($name:expr => $cfg:expr,)*) => {
                $(if $matches.is_present($name) {
                    $cfg = Some(true);
                })*
            };
        }

        // bool
        populate_config_bool!(matches;
            "force vanilla layout" => patch_config.force_vanilla_layout,
            "qol game breaking" => patch_config.preferences.qol_game_breaking,
            "qol cosmetic" => patch_config.preferences.qol_cosmetic,
            "qol scans" => patch_config.preferences.qol_pickup_scans,
            "qol general" => patch_config.preferences.qol_general,
            "automatic crash screen" => patch_config.preferences.automatic_crash_screen,
            "quickplay" => patch_config.preferences.quickplay,
            "skip_splash_screens" => patch_config.preferences.skip_splash_screens,
            "quickpatch" => patch_config.preferences.quickpatch,
            "quiet" => patch_config.preferences.quiet,
            "nonvaria heat damage" => patch_config.game_config.nonvaria_heat_damage,
            "auto enabled elevators" => patch_config.game_config.auto_enabled_elevators,
            "spring ball" => patch_config.game_config.spring_ball,
            "warp to start" => patch_config.game_config.warp_to_start,
        );
        if matches.is_present("staggered suit damage") {
            patch_config.game_config.staggered_suit_damage = Some(SuitDamageReduction::Default);
        }

        // string
        if let Some(input_iso_path) = matches.value_of("input iso path") {
            patch_config.input_iso = Some(input_iso_path.to_string());
        }
        if let Some(output_iso_path) = matches.value_of("output iso path") {
            patch_config.output_iso = Some(output_iso_path.to_string());
        }
        if let Some(extern_assets_dir) = matches.value_of("extern assets dir") {
            patch_config.extern_assets_dir = Some(extern_assets_dir.to_string());
        }
        if let Some(map_default_state) = matches.value_of("map default state") {
            patch_config.preferences.map_default_state = Some(map_default_state.to_string());
        }
        if let Some(artifact_behavior) = matches.value_of("artifact hint behavior") {
            patch_config.preferences.artifact_hint_behavior = Some(artifact_behavior.to_string());
        }
        if let Some(trilogy_disc_path) = matches.value_of("trilogy disc path") {
            patch_config.preferences.trilogy_disc_path = Some(trilogy_disc_path.to_string());
        }
        if let Some(starting_room) = matches.value_of("starting room") {
            patch_config.game_config.starting_room = Some(starting_room.to_string());
        }
        if let Some(qol_cutscenes) = matches.value_of("qol cutscenes") {
            patch_config.preferences.qol_cutscenes = Some(qol_cutscenes.to_string());
        }
        if let Some(phazon_dmg_mod) = matches.value_of("phazon damage modifier") {
            patch_config.game_config.phazon_damage_modifier = Some(phazon_dmg_mod.to_string());
        }
        if let Some(run_mode) = matches.value_of("run mode") {
            patch_config.run_mode = Some(run_mode.to_string());
        }
        if let Some(spring_ball_item_str) = matches.value_of("spring ball item") {
            patch_config.game_config.spring_ball_item = Some(spring_ball_item_str.to_string());
        }

        // integer/float
        if let Some(s) = matches.value_of("seed") {
            patch_config.seed = Some(s.parse::<u64>().unwrap());
        }
        if let Some(damage) = matches.value_of("heat damage per sec") {
            patch_config.game_config.heat_damage_per_sec = Some(damage.parse::<f32>().unwrap());
        }
        if let Some(damage) = matches.value_of("poison damage per sec") {
            patch_config.game_config.poison_damage_per_sec = Some(damage.parse::<f32>().unwrap());
        }
        if let Some(damage) = matches.value_of("phazon damage per sec") {
            patch_config.game_config.phazon_damage_per_sec = Some(damage.parse::<f32>().unwrap());
        }
        if let Some(etank_capacity) = matches.value_of("etank capacity") {
            patch_config.game_config.etank_capacity = Some(etank_capacity.parse::<u32>().unwrap());
        }
        if let Some(warp_to_start_delay_s) = matches.value_of("warp to start delay") {
            patch_config.game_config.warp_to_start_delay_s =
                Some(warp_to_start_delay_s.parse::<f32>().unwrap());
        }

        // custom
        if let Some(starting_items_str) = matches.value_of("starting items") {
            patch_config.game_config.starting_items = Some(StartingItems::from_u64(
                starting_items_str.parse::<u64>().unwrap(),
            ));
        }
        if let Some(item_loss_items_str) = matches.value_of("item loss items") {
            patch_config.game_config.item_loss_items = Some(StartingItems::from_u64(
                item_loss_items_str.parse::<u64>().unwrap(),
            ));
        }

        patch_config.parse()
    }
}

fn merge_json(config: &mut PatchConfigPrivate, text: &'static str) -> Result<(), String> {
    let data = serde_json::from_str(text);
    let data: PatchConfigPrivate = data.map_err(|e| format!("JSON parse failed: {}", e))?;
    config.merge(data);

    Ok(())
}

impl PatchConfigPrivate {
    // returns all non-vanilla game layers which this config modifies
    fn layers(&self) -> HashMap<u32, HashSet<u32>> {
        let mut layers = HashMap::new();

        for world in World::iter() {
            let world_key = world.to_json_key();
            let level_config = &self.level_data.get(world_key);
            if level_config.is_none() {
                continue;
            }
            let rooms = &level_config.unwrap().rooms;

            for room_config in rooms.values() {
                if room_config.special_functions.is_none() {
                    continue;
                }

                let special_functions = room_config.special_functions.as_ref().unwrap();
                for special_function in special_functions {
                    if special_function.type_ != SpecialFunctionType::ScriptLayerController
                        || special_function.layer_change_room_id.is_none()
                        || special_function.layer_change_layer_id.is_none()
                    {
                        continue; // not a layer change special function
                    }

                    let layer = special_function.layer_change_layer_id.unwrap();
                    let internal_id = special_function.layer_change_room_id.unwrap();

                    let room_lookup = ROOM_BY_INTERNAL_ID[&internal_id];

                    if layer < room_lookup.layer_count {
                        continue; // This is modifying a vanilla layer, so skip
                    }

                    layers
                        .entry(room_lookup.mrea_id)
                        .or_insert_with(HashSet::new);

                    let room = layers.get_mut(&room_lookup.mrea_id).unwrap();
                    room.insert(layer);
                }
            }
        }

        layers
    }

    /* Extends the "stuff" added/edited in each room */
    pub fn merge(&mut self, other: Self) {
        /* First check if there for any conflicts when adding new layers */
        let self_layers = self.layers();
        let other_layers = other.layers();

        for (mrea_id, other_room_layers) in other_layers {
            if !self_layers.contains_key(&mrea_id) {
                continue; // this room isn't modified by the current configuration
            }

            let self_room_layers = self_layers.get(&mrea_id).unwrap();

            if !self_room_layers.is_disjoint(&other_room_layers) {
                let subset: HashSet<_> = self_room_layers
                    .intersection(&other_room_layers)
                    .cloned()
                    .collect();
                panic!("Room 0x{:X} contains conflicting usage of new layers. The following layer IDs must not be used to resolve this conflict: {:?}", mrea_id, subset);
            }
        }

        /* Merge one room at a time */
        for world in World::iter() {
            let world_key = world.to_json_key();

            if !other.level_data.contains_key(world_key) {
                continue;
            }

            if !self.level_data.contains_key(world_key) {
                self.level_data
                    .insert(world_key.to_string(), LevelConfig::default());
            }

            let self_rooms = &mut self.level_data.get_mut(world_key).unwrap().rooms;
            let other_rooms = &other.level_data.get(world_key).unwrap().rooms;

            for (room_name, other_room_config) in other_rooms {
                if !self_rooms.contains_key(room_name) {
                    self_rooms.insert(room_name.to_string(), RoomConfig::default());
                }

                let self_room_config = self_rooms.get_mut(room_name).unwrap();

                extend_option_vec!(liquids, self_room_config, other_room_config);
                extend_option_vec!(pickups, self_room_config, other_room_config);
                extend_option_vec!(extra_scans, self_room_config, other_room_config);
                extend_option_vec!(platforms, self_room_config, other_room_config);
                extend_option_vec!(camera_hints, self_room_config, other_room_config);
                extend_option_vec!(blocks, self_room_config, other_room_config);
                extend_option_vec!(lock_on_points, self_room_config, other_room_config);
                extend_option_vec!(escape_sequences, self_room_config, other_room_config);
                extend_option_vec!(repositions, self_room_config, other_room_config);
                extend_option_vec!(hudmemos, self_room_config, other_room_config);
                extend_option_vec!(delete_ids, self_room_config, other_room_config);
                extend_option_vec!(add_connections, self_room_config, other_room_config);
                extend_option_vec!(remove_connections, self_room_config, other_room_config);
                extend_option_vec!(relays, self_room_config, other_room_config);
                extend_option_vec!(cutscene_skip_fns, self_room_config, other_room_config);
                extend_option_vec!(timers, self_room_config, other_room_config);
                extend_option_vec!(actor_keyframes, self_room_config, other_room_config);
                extend_option_vec!(spawn_points, self_room_config, other_room_config);
                extend_option_vec!(triggers, self_room_config, other_room_config);
                extend_option_vec!(special_functions, self_room_config, other_room_config);
                extend_option_vec!(actor_rotates, self_room_config, other_room_config);
                extend_option_vec!(streamed_audios, self_room_config, other_room_config);
                extend_option_vec!(waypoints, self_room_config, other_room_config);
                extend_option_vec!(counters, self_room_config, other_room_config);
                extend_option_vec!(switches, self_room_config, other_room_config);
                extend_option_vec!(player_hints, self_room_config, other_room_config);
                extend_option_vec!(distance_fogs, self_room_config, other_room_config);
                extend_option_vec!(bomb_slots, self_room_config, other_room_config);
                extend_option_vec!(controller_actions, self_room_config, other_room_config);
                extend_option_vec!(player_actors, self_room_config, other_room_config);
                extend_option_vec!(world_light_faders, self_room_config, other_room_config);
                extend_option_vec!(cameras, self_room_config, other_room_config);
                extend_option_vec!(camera_waypoints, self_room_config, other_room_config);
                extend_option_vec!(camera_filter_keyframes, self_room_config, other_room_config);
                extend_option_vec!(new_camera_hints, self_room_config, other_room_config);
                extend_option_vec!(camera_hint_triggers, self_room_config, other_room_config);

                if let Some(other_layers) = &other_room_config.layers {
                    if self_room_config.layers.is_none() {
                        self_room_config.layers = Some(HashMap::new());
                    }

                    let self_layers = self_room_config.layers.as_mut().unwrap();
                    for (layer, other_state) in other_layers {
                        match self_layers.get_mut(layer) {
                            Some(self_state) => {
                                if self_state != other_state {
                                    panic!(
                                        "Conflicting enable/disable state for Layer {} in {} - {}",
                                        layer, world_key, room_name
                                    );
                                }
                            }
                            None => {
                                self_layers.insert(*layer, *other_state);
                            }
                        }
                    }
                }

                if let Some(other_edit_objs) = &other_room_config.edit_objs {
                    if self_room_config.edit_objs.is_none() {
                        self_room_config.edit_objs = Some(HashMap::new());
                    }

                    let self_edit_objs = self_room_config.edit_objs.as_mut().unwrap();

                    for (id, other_config) in other_edit_objs {
                        match self_edit_objs.get_mut(id) {
                            Some(self_config) => {
                                // merge
                                merge_optional!(layer, self_config, other_config, room_name);
                                merge_optional!(position, self_config, other_config, room_name);
                                merge_optional!(rotation, self_config, other_config, room_name);
                                merge_optional!(scale, self_config, other_config, room_name);
                                merge_optional!(size, self_config, other_config, room_name);
                                merge_optional!(speed, self_config, other_config, room_name);
                                merge_optional!(damage, self_config, other_config, room_name);
                                merge_optional!(
                                    detection_range,
                                    self_config,
                                    other_config,
                                    room_name
                                );
                                merge_optional!(attack_range, self_config, other_config, room_name);
                                merge_optional!(health, self_config, other_config, room_name);

                                if let Some(other_vuln) = &other_config.vulnerability {
                                    match &self_config.vulnerability {
                                        Some(self_vuln) => {
                                            if DoorType::from_string(other_vuln.to_string())
                                                != DoorType::from_string(self_vuln.to_string())
                                            {
                                                panic!("Conflict in {}'s editObjs", room_name);
                                            }
                                        }
                                        None => {
                                            self_config.vulnerability =
                                                Some(other_vuln.to_string());
                                        }
                                    }
                                }

                                if let Some(other_vulns) = &other_config.vulnerabilities {
                                    match self_config.vulnerabilities.as_mut() {
                                        Some(self_vulns) => {
                                            for (idx, other_vuln) in other_vulns {
                                                match self_vulns.get_mut(idx) {
                                                    Some(self_vuln) => {
                                                        if DoorType::from_string(
                                                            other_vuln.to_string(),
                                                        ) != DoorType::from_string(
                                                            self_vuln.to_string(),
                                                        ) {
                                                            panic!(
                                                                "Conflict in {}'s editObjs",
                                                                room_name
                                                            );
                                                        }
                                                    }
                                                    None => {
                                                        self_vulns
                                                            .insert(*idx, other_vuln.to_string());
                                                    }
                                                }
                                            }
                                        }
                                        None => {
                                            self_config.vulnerabilities = Some(other_vulns.clone());
                                        }
                                    }
                                }

                                if let Some(other_healths) = &other_config.healths {
                                    match self_config.healths.as_mut() {
                                        Some(self_healths) => {
                                            for (idx, other_health) in other_healths {
                                                match self_healths.get_mut(idx) {
                                                    Some(self_health) => {
                                                        if self_health != other_health {
                                                            panic!(
                                                                "Conflict in {}'s editObjs",
                                                                room_name
                                                            );
                                                        }
                                                    }
                                                    None => {
                                                        self_healths.insert(*idx, *other_health);
                                                    }
                                                }
                                            }
                                        }
                                        None => {
                                            self_config.healths = Some(other_healths.clone());
                                        }
                                    }
                                }
                            }
                            None => {
                                // copy
                                self_edit_objs.insert(*id, other_config.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // parse and then handle configuration macros (e.g. a bool loading in several pages of JSON changes)
    fn parse(&self) -> Result<PatchConfig, String> {
        // Parse version
        let version = {
            let input_iso_path = self.input_iso.as_deref().unwrap_or("prime.iso");
            let input_iso_file = File::open(input_iso_path.trim())
                .map_err(|e| format!("Failed to open {}: {}", input_iso_path, e))?;
            let input_iso = unsafe { memmap::Mmap::map(&input_iso_file) }
                .map_err(|e| format!("Failed to open {}: {}", input_iso_path, e))?;

            let mut reader = Reader::new(&input_iso[..]);
            let gc_disc: structs::GcDisc = reader.read(());

            match (&gc_disc.header.game_identifier(), gc_disc.header.disc_id, gc_disc.header.version) {
                (b"GM8E01", 0, 0)  => Version::NtscU0_00,
                (b"GM8E01", 0, 1)  => Version::NtscU0_01,
                (b"GM8E01", 0, 2)  => Version::NtscU0_02,
                (b"GM8E01", 0, 48) => Version::NtscK,
                (b"GM8J01", 0, 0)  => Version::NtscJ,
                (b"GM8P01", 0, 0)  => Version::Pal,
                (b"R3ME01", 0, 0)  => Version::NtscUTrilogy,
                (b"R3IJ01", 0, 0)  => Version::NtscJTrilogy,
                (b"R3MP01", 0, 0)  => Version::PalTrilogy,
                _ => Err(concat!(
                        "The input ISO doesn't appear to be NTSC-US, NTSC-J, NTSC-K, PAL Metroid Prime, ",
                        "or NTSC-US, NTSC-J, PAL Metroid Prime Trilogy."
                    ))?
            }
        };

        let force_vanilla_layout = self.force_vanilla_layout.unwrap_or(false);

        let mut result = self.clone();

        let mode = result
            .preferences
            .qol_cutscenes
            .as_ref()
            .unwrap_or(&"original".to_string())
            .to_lowercase();
        let mode = mode.trim();

        if ["skippable", "skippablecompetitive"].contains(&mode) {
            merge_json(&mut result, SKIPPABLE_CUTSCENES)?;

            if [
                Version::NtscJ,
                Version::Pal,
                Version::NtscUTrilogy,
                Version::NtscJTrilogy,
                Version::PalTrilogy,
            ]
            .contains(&version)
            {
                merge_json(&mut result, SKIPPABLE_CUTSCENES_PAL)?;
            }

            if mode == "skippablecompetitive" {
                merge_json(&mut result, SKIPPABLE_CUTSCENES_COMPETITIVE)?;
            }
        }

        if self
            .preferences
            .qol_general
            .unwrap_or(!force_vanilla_layout)
        {
            merge_json(&mut result, QOL)?;
        }

        result.parse_inner(version)
    }

    fn parse_inner(&self, version: Version) -> Result<PatchConfig, String> {
        let run_mode = {
            if self.run_mode.is_some() {
                match self.run_mode.as_ref().unwrap().to_lowercase().trim() {
                    "create_iso" => RunMode::CreateIso,
                    "export_logbook" => RunMode::ExportLogbook,
                    "export_assets" => RunMode::ExportAssets,
                    _ => panic!("Unsupported run mode: {}", self.run_mode.as_ref().unwrap()),
                }
            } else {
                RunMode::CreateIso
            }
        };

        let input_iso_path = self.input_iso.as_deref().unwrap_or("prime.iso");
        let input_iso_file = File::open(input_iso_path.trim())
            .map_err(|e| format!("Failed to open {}: {}", input_iso_path, e))?;

        let input_iso = unsafe { memmap::Mmap::map(&input_iso_file) }
            .map_err(|e| format!("Failed to open {}: {}", input_iso_path, e))?;

        let output_iso_path = self.output_iso.as_deref().unwrap_or("prime_out.iso");

        let output_iso = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_iso_path)
            .map_err(|e| format!("Failed to open {}: {}", output_iso_path, e))?;

        let iso_format = if output_iso_path.ends_with(".gcz") {
            IsoFormat::Gcz
        } else if output_iso_path.ends_with(".ciso") {
            IsoFormat::Ciso
        } else {
            IsoFormat::Iso
        };

        let force_vanilla_layout = self.force_vanilla_layout.unwrap_or(false);

        let artifact_hint_behavior = {
            let artifact_hint_behavior_string = self
                .preferences
                .artifact_hint_behavior
                .as_deref()
                .unwrap_or("all")
                .trim()
                .to_lowercase();
            if artifact_hint_behavior_string == "all" {
                ArtifactHintBehavior::All
            } else if artifact_hint_behavior_string == "none" {
                ArtifactHintBehavior::None
            } else if artifact_hint_behavior_string == "default" {
                ArtifactHintBehavior::Default
            } else {
                Err(format!(
                    "Unhandled artifact hint behavior - '{}'",
                    artifact_hint_behavior_string
                ))?
            }
        };

        let map_default_state = {
            let map_default_state_string = self
                .preferences
                .map_default_state
                .as_deref()
                .unwrap_or("default")
                .trim()
                .to_lowercase();

            MapaObjectVisibilityMode::from_str(map_default_state_string.as_str())
                .ok()
                .unwrap_or_else(|| panic!("Invalid mapDefaultState '{}'", map_default_state_string))
        };

        let flaahgra_music_files = self
            .preferences
            .trilogy_disc_path
            .as_ref()
            .map(|path| extract_flaahgra_music_files(path))
            .transpose()?;

        let mut item_max_capacity = match &self.game_config.item_max_capacity {
            Some(max_capacity) => max_capacity
                .iter()
                .map(|(name, capacity)| (PickupType::from_str(name), *capacity))
                .collect(),
            None => HashMap::new(),
        };
        if !item_max_capacity.contains_key(&PickupType::EnergyTank) && !force_vanilla_layout {
            item_max_capacity.insert(PickupType::EnergyTank, 200);
        }
        item_max_capacity
            .entry(PickupType::UnknownItem2)
            .or_insert(2147483647);

        if item_max_capacity.contains_key(&PickupType::UnlimitedMissiles)
            || item_max_capacity.contains_key(&PickupType::UnlimitedPowerBombs)
            || item_max_capacity.contains_key(&PickupType::MissileLauncher)
            || item_max_capacity.contains_key(&PickupType::PowerBombLauncher)
            || item_max_capacity.contains_key(&PickupType::SpringBall)
            || item_max_capacity.contains_key(&PickupType::Nothing)
            || item_max_capacity.contains_key(&PickupType::FloatyJump)
            || item_max_capacity.contains_key(&PickupType::IceTrap)
        {
            panic!("Illegal pickup name in 'itemMaxCapacity'");
        }

        let qol_game_breaking = self
            .preferences
            .qol_game_breaking
            .unwrap_or(!force_vanilla_layout);
        let qol_cosmetic = self
            .preferences
            .qol_cosmetic
            .unwrap_or(!force_vanilla_layout);
        let qol_pickup_scans = self
            .preferences
            .qol_pickup_scans
            .unwrap_or(!force_vanilla_layout);
        let qol_general = self
            .preferences
            .qol_general
            .unwrap_or(!force_vanilla_layout);
        let qol_cutscenes = match self
            .preferences
            .qol_cutscenes
            .as_ref()
            .unwrap_or(&"original".to_string())
            .to_lowercase()
            .trim()
        {
            "original" => CutsceneMode::Original,
            "competitive" => CutsceneMode::Competitive,
            "skippable" => CutsceneMode::Skippable,
            "skippablecompetitive" => CutsceneMode::SkippableCompetitive,
            "minor" => CutsceneMode::Minor,
            "major" => CutsceneMode::Major,
            _ => panic!(
                "Unknown cutscene mode {}",
                self.preferences.qol_cutscenes.as_ref().unwrap()
            ),
        };

        let starting_room = {
            let room = self.game_config.starting_room.as_ref();
            match room {
                Some(room) => room.to_string(),
                None => {
                    if force_vanilla_layout {
                        "Frigate:Exterior Docking Hangar".to_string()
                    } else {
                        "Tallon:Landing Site".to_string()
                    }
                }
            }
        };

        let starting_items = {
            let items = self.game_config.starting_items.as_ref();

            match items {
                Some(items) => items.clone(),
                None => {
                    if force_vanilla_layout {
                        // from u64 2188378143
                        StartingItems {
                            power_beam: true,
                            combat_visor: true,
                            scan_visor: true,
                            missiles: 15,
                            energy_tanks: 0,
                            power_bombs: 0,
                            wave: false,
                            ice: false,
                            plasma: false,
                            charge: true,
                            morph_ball: true,
                            bombs: true,
                            spider_ball: false,
                            boost_ball: false,
                            power_suit: 0,
                            varia_suit: true,
                            gravity_suit: false,
                            phazon_suit: false,
                            thermal_visor: false,
                            xray: false,
                            space_jump: false,
                            grapple: true,
                            super_missile: false,
                            wavebuster: false,
                            ice_spreader: false,
                            flamethrower: false,
                            unknown_item_1: 0,
                            unlimited_missiles: false,
                            unlimited_power_bombs: false,
                            missile_launcher: true,
                            power_bomb_launcher: true,
                            spring_ball: false,
                        }
                    } else {
                        StartingItems::default()
                    }
                }
            }
        };

        let default_starting_visor = if starting_items.combat_visor {
            "combat"
        } else if starting_items.thermal_visor {
            "thermal"
        } else if starting_items.xray {
            "xray"
        } else {
            "scan"
        };

        let starting_visor = match self
            .game_config
            .starting_visor
            .as_ref()
            .unwrap_or(&default_starting_visor.to_string())
            .to_lowercase()
            .trim()
        {
            "combat" => Visor::Combat,
            "scan" => Visor::Scan,
            "thermal" => Visor::Thermal,
            "xray" => Visor::XRay,
            _ => panic!(
                "Unknown starting visor {}",
                self.game_config.starting_visor.as_ref().unwrap()
            ),
        };

        let default_starting_beam = if starting_items.power_beam {
            "power"
        } else if starting_items.plasma {
            "plasma"
        } else if starting_items.ice {
            "ice"
        } else if starting_items.wave {
            "wave"
        } else {
            "power"
        };

        let starting_beam = match self
            .game_config
            .starting_beam
            .as_ref()
            .unwrap_or(&default_starting_beam.to_string())
            .to_lowercase()
            .trim()
        {
            "power" => Beam::Power,
            "ice" => Beam::Ice,
            "wave" => Beam::Wave,
            "plasma" => Beam::Plasma,
            _ => panic!(
                "Unknown starting beam {}",
                self.game_config.starting_beam.as_ref().unwrap()
            ),
        };

        let spring_ball = self.game_config.spring_ball.unwrap_or(false);
        let warp_to_start = self.game_config.warp_to_start.unwrap_or(false);
        let main_menu_message = {
            let message = self.game_config.main_menu_message.as_ref();

            match message {
                Some(message) => message.to_string(),
                None => {
                    if force_vanilla_layout {
                        "".to_string()
                    } else {
                        "randomprime".to_string()
                    }
                }
            }
        };

        let credits_string = {
            let message = self.game_config.credits_string.as_ref();

            match message {
                Some(message) => Some(message.to_string()),
                None => {
                    if force_vanilla_layout {
                        Some("".to_string())
                    } else {
                        None
                    }
                }
            }
        };

        let results_string = {
            let message = self.game_config.results_string.as_ref();

            match message {
                Some(message) => Some(message.to_string()),
                None => {
                    if force_vanilla_layout {
                        Some("".to_string())
                    } else {
                        None
                    }
                }
            }
        };

        let phazon_damage_modifier = {
            let map_default_state_string = self
                .game_config
                .phazon_damage_modifier
                .as_deref()
                .unwrap_or("default")
                .trim()
                .to_lowercase();
            match &map_default_state_string[..] {
                "default" => PhazonDamageModifier::Default,
                "linear_delayed" => PhazonDamageModifier::LinearDelayed,
                "linear" => PhazonDamageModifier::Linear,
                _ => Err(format!(
                    "Unhandled phazon damage modifier - '{}'",
                    map_default_state_string
                ))?,
            }
        };

        let spring_ball_item = {
            match self.game_config.spring_ball_item.as_deref() {
                Some(s) => PickupType::from_str(s),
                None => PickupType::from_str({
                    match spring_ball {
                        false => "Spring Ball",
                        true => "Morph Ball Bomb",
                    }
                }),
            }
        };

        if spring_ball_item as u32 > PickupType::Nothing as u32 {
            panic!("Spring ball item cannot be {}!", spring_ball_item.name())
        }

        let result = PatchConfig {
            run_mode,
            logbook_filename: self.logbook_filename.clone(),
            export_asset_dir: self.export_asset_dir.clone(),
            version,
            input_iso,
            iso_format,
            output_iso,
            force_vanilla_layout,

            seed: self.seed.unwrap_or(123),
            uuid: self.uuid,
            extern_assets_dir: self.extern_assets_dir.clone(),

            level_data: self.level_data.clone(),
            strg: self.strg.clone(),

            qol_game_breaking,
            qol_cosmetic,
            qol_cutscenes,
            qol_pickup_scans,
            qol_general,

            phazon_elite_without_dynamo: self
                .game_config
                .phazon_elite_without_dynamo
                .unwrap_or(!force_vanilla_layout),
            main_plaza_door: self
                .game_config
                .main_plaza_door
                .unwrap_or(!force_vanilla_layout),
            backwards_labs: self
                .game_config
                .backwards_labs
                .unwrap_or(!force_vanilla_layout),
            backwards_frigate: self
                .game_config
                .backwards_frigate
                .unwrap_or(!force_vanilla_layout),
            backwards_upper_mines: self
                .game_config
                .backwards_upper_mines
                .unwrap_or(!force_vanilla_layout),
            backwards_lower_mines: self.game_config.backwards_lower_mines.unwrap_or(false),
            patch_power_conduits: self.game_config.patch_power_conduits.unwrap_or(false),
            remove_mine_security_station_locks: self
                .game_config
                .remove_mine_security_station_locks
                .unwrap_or(false),
            remove_hive_mecha: self.game_config.remove_hive_mecha.unwrap_or(false),
            power_bomb_arboretum_sandstone: self
                .game_config
                .power_bomb_arboretum_sandstone
                .unwrap_or(false),

            incinerator_drone_config: self.game_config.incinerator_drone_config.clone(),
            maze_seeds: self.game_config.maze_seeds.clone(),
            hall_of_the_elders_bomb_slot_covers: self
                .game_config
                .hall_of_the_elders_bomb_slot_covers,
            automatic_crash_screen: self.preferences.automatic_crash_screen.unwrap_or(true),
            visible_bounding_box: self.preferences.visible_bounding_box.unwrap_or(false),
            door_destination_scans: self
                .preferences
                .door_destination_scans
                .unwrap_or(!force_vanilla_layout),
            no_hud: self.preferences.no_hud.unwrap_or(false),
            artifact_hint_behavior,
            flaahgra_music_files,
            suit_colors: self.preferences.suit_colors.clone(),
            force_fusion: self.preferences.force_fusion.unwrap_or(false),
            cache_dir: self
                .preferences
                .cache_dir
                .clone()
                .unwrap_or("cache".to_string()),
            skip_splash_screens: self.preferences.skip_splash_screens.unwrap_or(false),
            default_game_options: self.preferences.default_game_options.clone(),
            quiet: self.preferences.quiet.unwrap_or(false),
            quickplay: self.preferences.quickplay.unwrap_or(false),
            quickpatch: self.preferences.quickpatch.unwrap_or(false),

            starting_room,
            starting_memo: self.game_config.starting_memo.clone(),
            spring_ball,
            spring_ball_item,
            warp_to_start,
            warp_to_start_delay_s: self.game_config.warp_to_start_delay_s.unwrap_or(0.0),

            shuffle_pickup_position: self.game_config.shuffle_pickup_position.unwrap_or(false),
            shuffle_pickup_pos_all_rooms: self
                .game_config
                .shuffle_pickup_pos_all_rooms
                .unwrap_or(false),
            remove_vanilla_blast_shields: self
                .game_config
                .remove_vanilla_blast_shields
                .unwrap_or(false),
            nonvaria_heat_damage: self.game_config.nonvaria_heat_damage.unwrap_or(false),
            staggered_suit_damage: self.game_config.staggered_suit_damage.unwrap_or_default(),
            heat_damage_per_sec: self.game_config.heat_damage_per_sec.unwrap_or(10.0),
            poison_damage_per_sec: self.game_config.poison_damage_per_sec.unwrap_or(0.11),
            phazon_damage_per_sec: self.game_config.phazon_damage_per_sec.unwrap_or(0.964),
            phazon_damage_modifier,
            auto_enabled_elevators: self.game_config.auto_enabled_elevators.unwrap_or(false),
            skip_ridley: self.game_config.skip_ridley.unwrap_or(false),
            multiworld_dol_patches: self.game_config.multiworld_dol_patches.unwrap_or(false),
            update_hint_state_replacement: self.game_config.update_hint_state_replacement.clone(),
            artifact_temple_layer_overrides: self
                .game_config
                .artifact_temple_layer_overrides
                .clone(),
            no_doors: self.game_config.no_doors.unwrap_or(false),
            boss_sizes: self.game_config.boss_sizes.clone().unwrap_or_default(),
            shoot_in_grapple: self.game_config.shoot_in_grapple.unwrap_or(false),
            difficulty_behavior: self
                .game_config
                .difficulty_behavior
                .unwrap_or(DifficultyBehavior::Either),
            legacy_block_size: self.game_config.legacy_block_size.unwrap_or(false),
            patch_wallcrawling: self.game_config.patch_wallcrawling.unwrap_or(false),
            blast_shield_lockon: self.game_config.blast_shield_lockon.unwrap_or(false),
            map_default_state,

            starting_items,
            item_loss_items: self
                .game_config
                .item_loss_items
                .clone()
                .unwrap_or_else(|| StartingItems::from_u64(1)),
            disable_item_loss: self.game_config.disable_item_loss.unwrap_or(true),
            escape_sequence_counts_up: self.game_config.escape_sequence_counts_up.unwrap_or(false),
            enable_ice_traps: self.game_config.enable_ice_traps.unwrap_or(true),
            missile_station_pb_refill: self.game_config.missile_station_pb_refill.unwrap_or(false),
            door_open_mode: self
                .game_config
                .door_open_mode
                .unwrap_or(DoorOpenMode::Original),
            starting_visor,
            starting_beam,

            etank_capacity: self.game_config.etank_capacity.unwrap_or(100),
            item_max_capacity,

            game_banner: self.game_config.game_banner.clone().unwrap_or_default(),
            comment: self.game_config.comment.clone().unwrap_or_default(),
            main_menu_message,

            credits_string,
            results_string,
            artifact_hints: self.game_config.artifact_hints.clone(),
            required_artifact_count: self.game_config.required_artifact_count,

            ctwk_config: self.tweaks.clone(),
        };

        Ok(result)
    }
}

/*** Helper Methods ***/

pub fn extract_flaahgra_music_files(iso_path: &str) -> Result<[Vec<u8>; 2], String> {
    let res = (|| {
        let dw = nod::Disc::new(iso_path)?;
        let mut partition = dw.open_partition(1)?;
        let meta = partition.meta()?;
        let fst = meta.fst()?;
        let file_r = read_file(partition.as_mut(), &fst, "MP1/Audio/rui_flaaghraR.dsp")?;
        let file_l = read_file(partition.as_mut(), &fst, "MP1/Audio/rui_flaaghraL.dsp")?;
        Ok([file_r, file_l])
    })();
    res.map_err(|err: Box<dyn std::error::Error>| {
        format!("Failed to extract Flaahgra music files: {}", err)
    })
}

fn read_file(
    partition: &mut dyn nod::PartitionBase,
    fst: &nod::Fst,
    path: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (_, node) = fst
        .find(path)
        .ok_or_else(|| format!("\"{path}\" not found"))?;
    let mut buf = Vec::new();
    partition.open_file(node)?.read_to_end(&mut buf)?;
    Ok(buf)
}
