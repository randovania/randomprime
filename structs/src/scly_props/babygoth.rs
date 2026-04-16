use auto_struct_macros::auto_struct;
use reader_writer::{generic_array::GenericArray, typenum::*, CStr};

use crate::{
    impl_patterned_info, impl_position, impl_rotation, impl_scale, scly_props::structs::*,
    SclyPropertyData,
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Babygoth<'r> {
    #[auto_struct(expect = 33)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,
    pub fireball_attack_time: f32, // TODO: speed
    pub fireball_attack_time_variance: f32,
    pub fireball_weapon: u32,
    pub fireball_damage: DamageInfo,
    pub attack_contact_damage: DamageInfo,
    pub fire_breath_weapon: u32,
    pub fire_breath_res: u32,
    pub fire_breath_damage: DamageInfo,
    pub mouth_vulnerability: DamageVulnerability,
    pub shell_vulnerability: DamageVulnerability,
    pub no_shell_model: u32,
    pub no_shell_skin: u32,
    pub shell_hit_points: f32,
    pub shell_crack_sfx: u32,
    pub intermediate_crack_particle: u32,
    pub crack_one_particle: u32,
    pub crack_two_particle: u32,
    pub destroy_shell_particle: u32,
    pub crack_one_sfx: u32,
    pub crack_two_sfx: u32,
    pub destroy_shell_sfx: u32,
    pub time_until_attack: f32,
    pub attack_cooldown_time: f32, // TODO: speed
    pub interest_time: f32,
    pub flame_player_steam_txtr: u32,
    pub flame_player_hit_sfx: u32,
    pub flame_player_ice_txtr: u32,
}

impl SclyPropertyData for Babygoth<'_> {
    const OBJECT_TYPE: u8 = 0x66;

    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info!();

    const SUPPORTS_DAMAGE_INFOS: bool = true;

    fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
        vec![
            self.patterned_info.contact_damage,
            self.fireball_damage,
            self.attack_contact_damage,
            self.fire_breath_damage,
        ]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.patterned_info.contact_damage = x[0];
        self.fireball_damage = x[1];
        self.attack_contact_damage = x[2];
        self.fire_breath_damage = x[3];
    }

    const SUPPORTS_VULNERABILITIES: bool = true;

    fn impl_get_vulnerabilities(&self) -> Vec<DamageVulnerability> {
        vec![
            self.patterned_info.damage_vulnerability.clone(),
            self.mouth_vulnerability.clone(),
            self.shell_vulnerability.clone(),
        ]
    }

    fn impl_set_vulnerabilities(&mut self, x: Vec<DamageVulnerability>) {
        self.patterned_info.damage_vulnerability = x[0].clone();
        self.mouth_vulnerability = x[1].clone();
        self.shell_vulnerability = x[2].clone();
    }
}
