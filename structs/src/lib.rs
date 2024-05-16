#![allow(ambiguous_glob_reexports, unused_imports)]
pub mod res_id;

mod ancs;
mod anim;
mod bnr;
mod cmdl;
mod ctwk;
mod dol;
mod dumb;
mod evnt;
mod font;
mod frme;
mod gc_disc;
mod hint;
mod mapa;
mod mapw;
pub mod mlvl;
mod mrea;
mod pak;
mod part;
mod savw;
mod scan;
mod scly;
mod strg;
mod thp;
mod txtr;

pub mod scly_props {
    // http://www.metroid2002.com/retromodding/wiki/User:Parax0/Sandbox
    pub mod actor;
    pub mod actor_key_frame;
    pub mod actor_rotate;
    pub mod ball_trigger;
    pub mod camera;
    pub mod camera_blur_keyframe;
    pub mod camera_filter_keyframe;
    pub mod camera_hint;
    pub mod camera_hint_trigger;
    pub mod controller_action;
    pub mod counter;
    pub mod damageable_trigger;
    pub mod distance_fog;
    pub mod dock;
    pub mod door;
    pub mod effect;
    pub mod grapple_point;
    pub mod hud_memo;
    pub mod memory_relay;
    pub mod new_camera_shaker;
    pub mod pickup;
    pub mod pickup_generator;
    pub mod platorm;
    pub mod player_actor;
    pub mod player_hint;
    pub mod point_of_interest;
    pub mod relay;
    pub mod snake_weed_swarm;
    pub mod sound;
    pub mod spawn_point;
    pub mod special_function;
    pub mod spider_ball_waypoint;
    pub mod streamed_audio;
    pub mod switch;
    pub mod timer;
    pub mod trigger;
    pub mod water;
    pub mod waypoint;
    pub mod world_light_fader;
    pub mod world_transporter;

    // "Generic" edit update
    pub mod ai_jump_point;
    pub mod ambient_ai;
    pub mod atomic_alpha;
    pub mod atomic_beta;
    pub mod babygoth;
    pub mod bloodflower;
    pub mod burrower;
    pub mod camera_pitch_volume;
    pub mod camera_waypoint;
    pub mod chozo_ghost;
    pub mod cover_point;
    pub mod debris;
    pub mod debris_extended;
    pub mod energy_ball;
    pub mod eyeball;
    pub mod fire_flea;
    pub mod fish_cloud;
    pub mod flaahgra_tentacle;
    pub mod flicker_bat;
    pub mod flying_pirate;
    pub mod geemer;
    pub mod gun_turret;
    pub mod jelly_zap;
    pub mod magdolite;
    pub mod metaree;
    pub mod metroid;
    pub mod metroid_beta;
    pub mod parasite;
    pub mod phazon_healing_nodule;
    pub mod phazon_pool;
    pub mod puddle_spore;
    pub mod puddle_toad_gamma;
    pub mod puffer;
    pub mod ripper;
    pub mod seedling;
    pub mod space_pirate;
    pub mod spank_weed;
    pub mod thardus_rock_projectile;
    pub mod tryclops;
    pub mod war_wasp;

    // bosses
    pub mod actor_contraption;
    pub mod beetle;
    pub mod drone;
    pub mod elite_pirate;
    pub mod flaahgra;
    pub mod ice_sheegoth;
    pub mod metroidprimestage1;
    pub mod metroidprimestage2;
    pub mod new_intro_boss;
    pub mod omega_pirate;
    pub mod ridley_v1;
    pub mod ridley_v2;
    pub mod thardus;

    pub mod structs;

    // "Generic" edit update
    pub use self::ai_jump_point::*;
    // bosses
    pub use self::beetle::*;
    pub use self::{
        actor::*, actor_contraption::*, actor_key_frame::*, actor_rotate::*, ambient_ai::*,
        atomic_alpha::*, atomic_beta::*, babygoth::*, ball_trigger::*, bloodflower::*, burrower::*,
        camera::*, camera_blur_keyframe::*, camera_filter_keyframe::*, camera_hint::*,
        camera_hint_trigger::*, camera_pitch_volume::*, camera_waypoint::*, chozo_ghost::*,
        controller_action::*, counter::*, cover_point::*, damageable_trigger::*, debris::*,
        debris_extended::*, distance_fog::*, dock::*, door::*, drone::*, effect::*,
        elite_pirate::*, energy_ball::*, eyeball::*, fire_flea::*, fish_cloud::*, flaahgra::*,
        flaahgra_tentacle::*, flicker_bat::*, flying_pirate::*, geemer::*, grapple_point::*,
        gun_turret::*, hud_memo::*, ice_sheegoth::*, jelly_zap::*, magdolite::*, memory_relay::*,
        metaree::*, metroid::*, metroid_beta::*, metroidprimestage1::*, metroidprimestage2::*,
        new_camera_shaker::*, new_intro_boss::*, omega_pirate::*, parasite::*,
        phazon_healing_nodule::*, phazon_pool::*, pickup::*, pickup_generator::*, platorm::*,
        player_actor::*, player_hint::*, point_of_interest::*, puddle_spore::*,
        puddle_toad_gamma::*, puffer::*, relay::*, ridley_v1::*, ridley_v2::*, ripper::*,
        seedling::*, snake_weed_swarm::*, sound::*, space_pirate::*, spank_weed::*, spawn_point::*,
        special_function::*, spider_ball_waypoint::*, streamed_audio::*, switch::*, thardus::*,
        thardus_rock_projectile::*, timer::*, trigger::*, tryclops::*, war_wasp::*, water::*,
        waypoint::*, world_light_fader::*, world_transporter::*,
    };
}

pub use ancs::*;
pub use anim::*;
pub use bnr::*;
pub use cmdl::*;
pub use ctwk::*;
pub use dol::*;
pub use dumb::*;
pub use evnt::*;
pub use font::*;
pub use frme::*;
pub use gc_disc::*;
pub use hint::*;
pub use mapa::*;
pub use mapw::*;
pub use mlvl::*;
pub use mrea::*;
pub use pak::*;
pub use part::*;
pub use res_id::ResId;
pub use savw::*;
pub use scan::*;
pub use scly::*;
// "Generic" edit update
pub use scly_props::ai_jump_point::*;
// bosses
pub use scly_props::beetle::*;
pub use scly_props::{
    actor::*, actor_contraption::*, actor_key_frame::*, actor_rotate::*, ambient_ai::*,
    atomic_alpha::*, atomic_beta::*, babygoth::*, ball_trigger::*, bloodflower::*, burrower::*,
    camera::*, camera_blur_keyframe::*, camera_filter_keyframe::*, camera_hint::*,
    camera_hint_trigger::*, camera_pitch_volume::*, camera_waypoint::*, chozo_ghost::*,
    controller_action::*, counter::*, cover_point::*, damageable_trigger::*, debris::*,
    debris_extended::*, distance_fog::*, dock::*, door::*, drone::*, effect::*, elite_pirate::*,
    energy_ball::*, eyeball::*, fire_flea::*, fish_cloud::*, flaahgra::*, flaahgra_tentacle::*,
    flicker_bat::*, flying_pirate::*, geemer::*, grapple_point::*, gun_turret::*, hud_memo::*,
    ice_sheegoth::*, jelly_zap::*, magdolite::*, memory_relay::*, metaree::*, metroid::*,
    metroid_beta::*, metroidprimestage1::*, metroidprimestage2::*, new_camera_shaker::*,
    new_intro_boss::*, omega_pirate::*, parasite::*, phazon_healing_nodule::*, phazon_pool::*,
    pickup::*, pickup_generator::*, platorm::*, player_actor::*, player_hint::*,
    point_of_interest::*, puddle_spore::*, puddle_toad_gamma::*, puffer::*, relay::*, ridley_v1::*,
    ridley_v2::*, ripper::*, seedling::*, snake_weed_swarm::*, sound::*, space_pirate::*,
    spank_weed::*, spawn_point::*, special_function::*, spider_ball_waypoint::*, streamed_audio::*,
    structs as scly_structs, switch::*, thardus::*, thardus_rock_projectile::*, timer::*,
    trigger::*, tryclops::*, war_wasp::*, water::*, waypoint::*, world_light_fader::*,
    world_transporter::*,
};
pub use strg::*;
pub use thp::*;
pub use txtr::*;
