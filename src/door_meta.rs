use reader_writer::FourCC;
use structs::{res_id, scly_structs::TypeVulnerability, ResId};

use crate::{
    custom_assets::custom_asset_ids,
    structs::scly_props::structs::{BeamCombos, ChargedBeams, DamageVulnerability},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum DoorType {
    Blue,
    Purple,
    White,
    Red,
    PowerOnly,
    PowerBomb,
    Bomb,
    Boost,
    Missile,
    Charge,
    Super,
    Wavebuster,
    Icespreader,
    Flamethrower,
    Ai,
    Disabled,
    Grapple,
    Phazon,
    Thermal,
    XRay,
    Scan,
    VerticalBlue,
    VerticalPowerOnly,
    VerticalPurple,
    VerticalWhite,
    VerticalRed,
    VerticalBoost,
    VerticalPowerBomb,
    VerticalBomb,
    VerticalMissile,
    VerticalCharge,
    VerticalSuper,
    VerticalDisabled,
    VerticalWavebuster,
    VerticalIcespreader,
    VerticalFlamethrower,
    VerticalAi,
    VerticalGrapple,
    VerticalPhazon,
    VerticalThermal,
    VerticalXRay,
    VerticalScan,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum BlastShieldType {
    Missile,
    PowerBomb,
    Super,
    Wavebuster,
    Icespreader,
    Flamethrower,
    Charge,
    Grapple,
    Bomb,
    Phazon,
    Thermal,
    XRay,
    Scan,

    // These don't have assets
    None,
    Unchanged,
}

impl DoorType {
    pub const fn is_vertical(&self) -> bool {
        matches!(
            self,
            DoorType::VerticalBlue
                | DoorType::VerticalPowerOnly
                | DoorType::VerticalPurple
                | DoorType::VerticalWhite
                | DoorType::VerticalRed
                | DoorType::VerticalBoost
                | DoorType::VerticalPowerBomb
                | DoorType::VerticalBomb
                | DoorType::VerticalMissile
                | DoorType::VerticalCharge
                | DoorType::VerticalSuper
                | DoorType::VerticalDisabled
                | DoorType::VerticalWavebuster
                | DoorType::VerticalIcespreader
                | DoorType::VerticalFlamethrower
                | DoorType::VerticalAi
                | DoorType::VerticalGrapple
                | DoorType::VerticalPhazon
                | DoorType::VerticalThermal
                | DoorType::VerticalXRay
                | DoorType::VerticalScan
        )
    }

    pub fn to_vertical(&self) -> DoorType {
        match self {
            DoorType::Blue => DoorType::VerticalBlue,
            DoorType::PowerOnly => DoorType::VerticalPowerOnly,
            DoorType::Purple => DoorType::VerticalPurple,
            DoorType::White => DoorType::VerticalWhite,
            DoorType::Red => DoorType::VerticalRed,
            DoorType::Boost => DoorType::VerticalBoost,
            DoorType::PowerBomb => DoorType::VerticalPowerBomb,
            DoorType::Bomb => DoorType::VerticalBomb,
            DoorType::Missile => DoorType::VerticalMissile,
            DoorType::Charge => DoorType::VerticalCharge,
            DoorType::Super => DoorType::VerticalSuper,
            DoorType::Disabled => DoorType::VerticalDisabled,
            DoorType::Wavebuster => DoorType::VerticalWavebuster,
            DoorType::Icespreader => DoorType::VerticalIcespreader,
            DoorType::Flamethrower => DoorType::VerticalFlamethrower,
            DoorType::Ai => DoorType::VerticalAi,
            DoorType::Grapple => DoorType::VerticalGrapple,
            DoorType::Phazon => DoorType::VerticalPhazon,
            DoorType::Thermal => DoorType::VerticalThermal,
            DoorType::XRay => DoorType::VerticalXRay,
            DoorType::Scan => DoorType::VerticalScan,
            _ => {
                if !self.is_vertical() {
                    panic!("no vertical door for type {:?}", self);
                }

                self.clone().to_owned()
            }
        }
    }

    pub fn to_horizontal(&self) -> DoorType {
        match self {
            DoorType::VerticalBlue => DoorType::Blue,
            DoorType::VerticalPowerOnly => DoorType::PowerOnly,
            DoorType::VerticalPurple => DoorType::Purple,
            DoorType::VerticalWhite => DoorType::White,
            DoorType::VerticalRed => DoorType::Red,
            DoorType::VerticalBoost => DoorType::Boost,
            DoorType::VerticalPowerBomb => DoorType::PowerBomb,
            DoorType::VerticalBomb => DoorType::Bomb,
            DoorType::VerticalMissile => DoorType::Missile,
            DoorType::VerticalCharge => DoorType::Charge,
            DoorType::VerticalSuper => DoorType::Super,
            DoorType::VerticalDisabled => DoorType::Disabled,
            DoorType::VerticalWavebuster => DoorType::Wavebuster,
            DoorType::VerticalIcespreader => DoorType::Icespreader,
            DoorType::VerticalFlamethrower => DoorType::Flamethrower,
            DoorType::VerticalAi => DoorType::Ai,
            DoorType::VerticalGrapple => DoorType::Grapple,
            DoorType::VerticalPhazon => DoorType::Phazon,
            DoorType::VerticalThermal => DoorType::Thermal,
            DoorType::VerticalXRay => DoorType::XRay,
            DoorType::VerticalScan => DoorType::Scan,
            _ => {
                if self.is_vertical() {
                    panic!("no horizontal door for type {:?}", self);
                }

                self.clone().to_owned()
            }
        }
    }

    pub fn to_primary_color(&self) -> DoorType {
        match self {
            DoorType::Blue => DoorType::Blue,
            DoorType::PowerOnly => DoorType::Blue,
            DoorType::Purple => DoorType::Purple,
            DoorType::White => DoorType::White,
            DoorType::Red => DoorType::Red,
            DoorType::Boost => DoorType::Blue,
            DoorType::PowerBomb => DoorType::Blue,
            DoorType::Bomb => DoorType::Blue,
            DoorType::Missile => DoorType::Blue,
            DoorType::Charge => DoorType::Blue,
            DoorType::Super => DoorType::Blue,
            DoorType::Disabled => DoorType::Disabled,
            DoorType::Wavebuster => DoorType::Purple,
            DoorType::Icespreader => DoorType::White,
            DoorType::Flamethrower => DoorType::Red,
            DoorType::Ai => DoorType::Blue,
            DoorType::Grapple => DoorType::Blue,
            DoorType::Phazon => DoorType::Blue,
            DoorType::Thermal => DoorType::Blue,
            DoorType::XRay => DoorType::Blue,
            DoorType::Scan => DoorType::Blue,
            _ => {
                if !self.is_vertical() {
                    panic!("unhandled door type {:?}", self);
                }

                self.to_horizontal().to_primary_color().to_vertical()
            }
        }
    }

    pub fn from_string(string: String) -> Option<Self> {
        let test_str = string.trim().to_lowercase().replace([' ', '_', '-'], "");
        let test_str = test_str.as_str();

        match test_str {
            "blue" => Some(DoorType::Blue),
            "poweronly" => Some(DoorType::PowerOnly),
            "powerbeamonly" => Some(DoorType::PowerOnly),
            "purple" => Some(DoorType::Purple),
            "wave" => Some(DoorType::Purple),
            "wavebeam" => Some(DoorType::Purple),
            "white" => Some(DoorType::White),
            "ice" => Some(DoorType::White),
            "icebeam" => Some(DoorType::White),
            "red" => Some(DoorType::Red),
            "plasma" => Some(DoorType::Red),
            "plasmabeam" => Some(DoorType::Red),
            "powerbomb" => Some(DoorType::PowerBomb),
            "bomb" => Some(DoorType::Bomb),
            "bombs" => Some(DoorType::Bomb),
            "missile" => Some(DoorType::Missile),
            "missiles" => Some(DoorType::Missile),
            "charge" => Some(DoorType::Charge),
            "chargebeam" => Some(DoorType::Charge),
            "super" => Some(DoorType::Super),
            "supermissile" => Some(DoorType::Super),
            "supermissiles" => Some(DoorType::Super),
            "disabled" => Some(DoorType::Disabled),
            "wavebuster" => Some(DoorType::Wavebuster),
            "icespreader" => Some(DoorType::Icespreader),
            "flamethrower" => Some(DoorType::Flamethrower),
            "ai" => Some(DoorType::Ai),
            "enemy" => Some(DoorType::Ai),
            "grapple" => Some(DoorType::Grapple),
            "grapplebeam" => Some(DoorType::Grapple),
            "phazon" => Some(DoorType::Phazon),
            "phazonbeam" => Some(DoorType::Phazon),
            "thermal" => Some(DoorType::Thermal),
            "thermalvisor" => Some(DoorType::Thermal),
            "xray" => Some(DoorType::XRay),
            "xrayvisor" => Some(DoorType::XRay),
            "scan" => Some(DoorType::Scan),
            "scanvisor" => Some(DoorType::Scan),
            _ => None,
        }
    }

    pub const fn shield_cmdl(&self) -> ResId<res_id::CMDL> {
        // model of door, includes specification for which 128x128 texture to line door frame with
        match self {
            DoorType::Blue => ResId::new(0x0734977A), // vanilla CMDL - "blueShield_v1" - door frame model
            DoorType::PowerOnly => custom_asset_ids::POWER_BEAM_CMDL,
            DoorType::Purple => ResId::new(0x33188D1B), // vanilla CMDL
            DoorType::White => ResId::new(0x59649E9D),  // vanilla CMDL
            DoorType::Red => ResId::new(0xBBBA1EC7),    // vanilla CMDL
            DoorType::Boost => ResId::new(0x0734977A),  // unused
            DoorType::PowerBomb => custom_asset_ids::POWER_BOMB_CMDL,
            DoorType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_CMDL,
            DoorType::Missile => custom_asset_ids::MISSILE_CMDL,
            DoorType::Charge => custom_asset_ids::CHARGE_BEAM_CMDL,
            DoorType::Super => custom_asset_ids::SUPER_MISSILE_CMDL,
            DoorType::Disabled => custom_asset_ids::DISABLED_CMDL,
            DoorType::Wavebuster => custom_asset_ids::WAVEBUSTER_CMDL,
            DoorType::Icespreader => custom_asset_ids::ICE_SPREADER_CMDL,
            DoorType::Flamethrower => custom_asset_ids::FLAMETHROWER_CMDL,
            DoorType::Ai => custom_asset_ids::AI_CMDL,
            DoorType::Grapple => custom_asset_ids::GRAPPLE_BEAM_CMDL,
            DoorType::Phazon => custom_asset_ids::PHAZON_BEAM_CMDL,
            DoorType::Thermal => custom_asset_ids::THERMAL_VISOR_CMDL,
            DoorType::XRay => custom_asset_ids::XRAY_VISOR_CMDL,
            DoorType::Scan => custom_asset_ids::SCAN_VISOR_CMDL,

            // vertical doors need a different CMDL, otherwise it will look like this: https://i.imgur.com/jGjWnmg.png //
            DoorType::VerticalBlue => ResId::new(0x18D0AEE6), // vanilla horizontal CMDL (blue)
            DoorType::VerticalPowerOnly => custom_asset_ids::POWER_BEAM_VERTICAL_CMDL,
            DoorType::VerticalPurple => ResId::new(0x095B0B93), // vanilla CMDL
            DoorType::VerticalWhite => ResId::new(0xB7A8A4C9),  // vanilla CMDL
            DoorType::VerticalRed => custom_asset_ids::PLASMA_VERTICAL_CMDL, // vanilla CMDL
            DoorType::VerticalBoost => custom_asset_ids::BOOST_VERTICAL_CMDL,
            DoorType::VerticalPowerBomb => custom_asset_ids::POWER_BOMB_VERTICAL_CMDL,
            DoorType::VerticalBomb => custom_asset_ids::MORPH_BALL_BOMBS_VERTICAL_CMDL,
            DoorType::VerticalMissile => custom_asset_ids::MISSILE_VERTICAL_CMDL,
            DoorType::VerticalCharge => custom_asset_ids::CHARGE_BEAM_VERTICAL_CMDL,
            DoorType::VerticalSuper => custom_asset_ids::SUPER_MISSILE_VERTICAL_CMDL,
            DoorType::VerticalDisabled => custom_asset_ids::DISABLED_VERTICAL_CMDL,
            DoorType::VerticalWavebuster => custom_asset_ids::WAVEBUSTER_VERTICAL_CMDL,
            DoorType::VerticalIcespreader => custom_asset_ids::ICE_SPREADER_VERTICAL_CMDL,
            DoorType::VerticalFlamethrower => custom_asset_ids::FLAMETHROWER_VERTICAL_CMDL,
            DoorType::VerticalAi => custom_asset_ids::AI_VERTICAL_CMDL,
            DoorType::VerticalGrapple => custom_asset_ids::GRAPPLE_BEAM_VERTICAL_CMDL,
            DoorType::VerticalPhazon => custom_asset_ids::PHAZON_BEAM_VERTICAL_CMDL,
            DoorType::VerticalThermal => custom_asset_ids::THERMAL_VISOR_VERTICAL_CMDL,
            DoorType::VerticalXRay => custom_asset_ids::XRAY_VISOR_VERTICAL_CMDL,
            DoorType::VerticalScan => custom_asset_ids::SCAN_VISOR_VERTICAL_CMDL,
        }
    }

    pub fn map_object_type(&self) -> u32 {
        let door = self.to_horizontal();

        match door {
            DoorType::Blue => structs::MapaObjectType::DoorNormal as u32,
            DoorType::PowerOnly => structs::MapaObjectType::DoorNormal as u32,
            DoorType::Charge => structs::MapaObjectType::DoorNormal as u32,
            DoorType::Bomb => structs::MapaObjectType::DoorNormal as u32,
            DoorType::Purple => structs::MapaObjectType::DoorWave as u32,
            DoorType::Wavebuster => structs::MapaObjectType::DoorWave as u32,
            DoorType::White => structs::MapaObjectType::DoorIce as u32,
            DoorType::Icespreader => structs::MapaObjectType::DoorIce as u32,
            DoorType::Red => structs::MapaObjectType::DoorPlasma as u32,
            DoorType::Flamethrower => structs::MapaObjectType::DoorPlasma as u32,
            _ => structs::MapaObjectType::DoorShield as u32, // everything else is non-vanilla and thus shield
        }
    }

    // The following three are the textures for the damageable trigger

    pub fn pattern0_txtr(&self) -> ResId<res_id::TXTR> {
        let door = self.to_horizontal();

        match door {
            DoorType::Blue => ResId::new(0x544A9892),      // testb.TXTR
            DoorType::PowerOnly => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Purple => ResId::new(0x544A9892),    // testb.TXTR
            DoorType::White => ResId::new(0x544A9892),     // testb.TXTR
            DoorType::Red => ResId::new(0x544A9892),       // testb.TXTR
            DoorType::Boost => ResId::new(0x544A9892),     // testb.TXTR
            DoorType::PowerBomb => custom_asset_ids::PINK_TXTR,
            DoorType::Bomb => custom_asset_ids::ORANGE_TXTR,
            DoorType::Missile => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Charge => custom_asset_ids::YELLOW_TXTR,
            DoorType::Super => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Disabled => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Wavebuster => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Icespreader => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Flamethrower => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Ai => ResId::new(0x544A9892),    // testb.TXTR
            DoorType::Grapple => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Phazon => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Thermal => ResId::new(0x544A9892), // testb.TXTR
            DoorType::XRay => ResId::new(0x544A9892),  // testb.TXTR
            DoorType::Scan => ResId::new(0x544A9892),  // testb.TXTR
            _ => panic!("Unhandled pattern0_txtr"),
        }
    }

    pub fn pattern1_txtr(&self) -> ResId<res_id::TXTR> {
        let door = self.to_horizontal();

        match door {
            DoorType::Blue => ResId::new(0x544A9892),      // testb.TXTR
            DoorType::PowerOnly => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Purple => ResId::new(0x544A9892),    // testb.TXTR
            DoorType::White => ResId::new(0x544A9892),     // testb.TXTR
            DoorType::Red => ResId::new(0x544A9892),       // testb.TXTR
            DoorType::Boost => ResId::new(0x544A9892),     // testb.TXTR
            DoorType::PowerBomb => ResId::new(0xcfa9dff3), // Thermal_Spot_2.TXTR
            DoorType::Bomb => ResId::new(0xcfa9dff3),      // Thermal_Spot_2.TXTR
            DoorType::Missile => ResId::new(0x544A9892),   // testb.TXTR
            DoorType::Charge => ResId::new(0xcfa9dff3),    // Thermal_Spot_2.TXTR
            DoorType::Super => ResId::new(0x544A9892),     // testb.TXTR
            DoorType::Disabled => ResId::new(0x544A9892),  // testb.TXTR
            DoorType::Wavebuster => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Icespreader => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Flamethrower => ResId::new(0x544A9892), // testb.TXTR
            DoorType::Ai => ResId::new(0x544A9892),        // testb.TXTR
            DoorType::Grapple => ResId::new(0x544A9892),   // testb.TXTR
            DoorType::Phazon => ResId::new(0x544A9892),    // testb.TXTR
            DoorType::Thermal => ResId::new(0x544A9892),   // testb.TXTR
            DoorType::XRay => ResId::new(0x544A9892),      // testb.TXTR
            DoorType::Scan => ResId::new(0x544A9892),      // testb.TXTR
            _ => panic!("Unhandled pattern1_txtr"),
        }
    }

    pub fn color_txtr(&self) -> ResId<res_id::TXTR> {
        let door = self.to_horizontal();

        match door {
            DoorType::Blue => ResId::new(0x8A7F3683), // vanilla blue
            DoorType::PowerOnly => ResId::new(0x1D588B22), // solid yellow
            DoorType::Purple => ResId::new(0xF68DF7F1), // vanilla purple
            DoorType::White => ResId::new(0xBE4CD99D), // vanilla white
            DoorType::Red => ResId::new(0xFC095F6C),  // vanilla red
            DoorType::Boost => ResId::new(0x8A7F3683), // vanilla blue
            DoorType::PowerBomb => custom_asset_ids::TESTBNEW_TXTR, // testbnew.txtr
            DoorType::Bomb => custom_asset_ids::TESTBNEW_TXTR, // testbnew.txtr
            DoorType::Missile => ResId::new(0x8344BEC8), // solid grey
            DoorType::Charge => custom_asset_ids::TESTBNEW_TXTR, // testbnew.txtr
            DoorType::Super => ResId::new(0xD5C17775), // solid green
            DoorType::Disabled => ResId::new(0x717AABCE), // void with specks
            DoorType::Wavebuster => ResId::new(0xF68DF7F1), // vanilla purple
            DoorType::Icespreader => ResId::new(0xBE4CD99D), // vanilla white
            DoorType::Flamethrower => ResId::new(0xFC095F6C), // vanilla red
            DoorType::Ai => ResId::new(0x717AABCE),   // void with specks
            DoorType::Grapple => ResId::new(0x8A7F3683), // vanilla blue
            DoorType::Phazon => ResId::new(0x8A7F3683), // vanilla blue
            DoorType::Thermal => ResId::new(0x8A7F3683), // vanilla blue
            DoorType::XRay => ResId::new(0x8A7F3683), // vanilla blue
            DoorType::Scan => ResId::new(0x8344BEC8), // solid grey
            _ => panic!("Unhandled color_txtr"),
        }
    }

    pub fn holorim_txtr(&self) -> ResId<res_id::TXTR> {
        // The the color applied from the rim of the door frame, specified in CMDL
        let door = self.to_horizontal();

        match door {
            DoorType::Blue => ResId::new(0x88ED4593), // vanilla TXTR - "blueholorim" texture [128x128]
            DoorType::PowerOnly => custom_asset_ids::POWER_BEAM_HOLORIM_TXTR,
            DoorType::Purple => ResId::new(0xAB031EA9), // vanilla TXTR
            DoorType::White => ResId::new(0xF6870C9F),  // vanilla TXTR
            DoorType::Red => ResId::new(0x61A6945B),    // vanilla TXTR
            DoorType::Boost => ResId::new(0x88ED4593),  // unused
            DoorType::PowerBomb => custom_asset_ids::POWER_BOMB_HOLORIM_TXTR,
            DoorType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_HOLORIM_TXTR,
            DoorType::Missile => ResId::new(0x459582C1), // "bedroomeyesC"
            DoorType::Charge => custom_asset_ids::CHARGE_BEAM_HOLORIM_TXTR,
            DoorType::Super => custom_asset_ids::SUPER_MISSILE_HOLORIM_TXTR,
            DoorType::Wavebuster => custom_asset_ids::WAVEBUSTER_HOLORIM_TXTR,
            DoorType::Icespreader => custom_asset_ids::ICE_SPREADER_HOLORIM_TXTR,
            DoorType::Flamethrower => custom_asset_ids::FLAMETHROWER_HOLORIM_TXTR,
            DoorType::Disabled => ResId::new(0x717AABCE), // void with specks
            DoorType::Ai => custom_asset_ids::AI_TXTR,
            DoorType::Grapple => custom_asset_ids::GRAPPLE_BEAM_HOLORIM_TXTR,
            DoorType::Phazon => custom_asset_ids::PHAZON_BEAM_HOLORIM_TXTR,
            DoorType::Thermal => custom_asset_ids::THERMAL_VISOR_HOLORIM_TXTR,
            DoorType::XRay => custom_asset_ids::XRAY_VISOR_HOLORIM_TXTR,
            DoorType::Scan => custom_asset_ids::SCAN_VISOR_HOLORIM_TXTR,
            _ => panic!("Unhandled holorim_txtr"),
        }
    }

    pub fn scan(&self) -> ResId<res_id::SCAN> {
        let door = self.to_horizontal();

        match door {
            DoorType::PowerOnly => custom_asset_ids::POWER_BEAM_SCAN,
            DoorType::Boost => custom_asset_ids::BOOST_SCAN,
            DoorType::PowerBomb => custom_asset_ids::POWER_BOMB_SCAN,
            DoorType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_SCAN,
            DoorType::Missile => custom_asset_ids::MISSILE_SCAN,
            DoorType::Charge => custom_asset_ids::CHARGE_BEAM_SCAN,
            DoorType::Super => custom_asset_ids::SUPER_MISSILE_SCAN,
            DoorType::Wavebuster => custom_asset_ids::WAVEBUSTER_SCAN,
            DoorType::Icespreader => custom_asset_ids::ICE_SPREADER_SCAN,
            DoorType::Flamethrower => custom_asset_ids::FLAMETHROWER_SCAN,
            DoorType::Disabled => custom_asset_ids::DISABLED_SCAN,
            DoorType::Ai => custom_asset_ids::AI_SCAN,
            DoorType::Grapple => ResId::invalid(), // door is just for color
            DoorType::Phazon => custom_asset_ids::PHAZON_BEAM_SCAN,
            DoorType::Thermal => ResId::invalid(), // door is just for color
            DoorType::XRay => ResId::invalid(),    // door is just for color
            DoorType::Scan => ResId::invalid(),    // door is just for color

            // Vanilla doors don't need scans //
            _ => ResId::invalid(),
        }
    }

    pub fn strg(&self) -> ResId<res_id::STRG> {
        let door = self.to_horizontal();

        match door {
            DoorType::PowerOnly => custom_asset_ids::POWER_BEAM_STRG,
            DoorType::Boost => custom_asset_ids::BOOST_STRG,
            DoorType::PowerBomb => custom_asset_ids::POWER_BOMB_STRG,
            DoorType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_STRG,
            DoorType::Missile => custom_asset_ids::MISSILE_STRG,
            DoorType::Charge => custom_asset_ids::CHARGE_BEAM_STRG,
            DoorType::Super => custom_asset_ids::SUPER_MISSILE_STRG,
            DoorType::Wavebuster => custom_asset_ids::WAVEBUSTER_STRG,
            DoorType::Icespreader => custom_asset_ids::ICE_SPREADER_STRG,
            DoorType::Flamethrower => custom_asset_ids::FLAMETHROWER_STRG,
            DoorType::Disabled => custom_asset_ids::DISABLED_STRG,
            DoorType::Ai => custom_asset_ids::AI_STRG,
            DoorType::Grapple => ResId::invalid(), // door is just for color
            DoorType::Phazon => custom_asset_ids::PHAZON_BEAM_STRG,
            DoorType::Thermal => ResId::invalid(), // door is just for color
            DoorType::XRay => ResId::invalid(),    // door is just for color
            DoorType::Scan => ResId::invalid(),    // door is just for color

            // Vanilla doors don't need scans //
            _ => ResId::invalid(),
        }
    }

    pub fn scan_text(&self) -> Vec<String> {
        let door = self.to_horizontal();

        match door {
            DoorType::PowerOnly => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will only open with &push;&main-color=#D91818;Power Beam&pop;.\0"
                    .to_string(),
            ],
            DoorType::Boost => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will open with &push;&main-color=#D91818;Boost Ball&pop;.\0".to_string(),
            ],
            DoorType::PowerBomb => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will open with &push;&main-color=#D91818;Power Bombs&pop;.\0"
                    .to_string(),
            ],
            DoorType::Bomb => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will open with &push;&main-color=#D91818;Morph Ball Bombs&pop;.\0"
                    .to_string(),
            ],
            DoorType::Missile => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will open with &push;&main-color=#D91818;Missiles&pop;.\0".to_string(),
            ],
            DoorType::Charge => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will open with &push;&main-color=#D91818;Charge Beam&pop;.\0"
                    .to_string(),
            ],
            DoorType::Super => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will open with &push;&main-color=#D91818;Super Missiles&pop;.\0"
                    .to_string(),
            ],
            DoorType::Wavebuster => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will open with &push;&main-color=#D91818;Wavebuster&pop;.\0".to_string(),
            ],
            DoorType::Icespreader => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will open with &push;&main-color=#D91818;Ice Spreader&pop;.\0"
                    .to_string(),
            ],
            DoorType::Flamethrower => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will open with &push;&main-color=#D91818;Flamethrower&pop;.\0"
                    .to_string(),
            ],
            DoorType::Disabled => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door cannot be opened.\0".to_string(),
            ],
            DoorType::Ai => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will open with the &push;&main-color=#D91818;help of an enemy&pop;.\0"
                    .to_string(),
            ],
            DoorType::Phazon => vec![
                "Analysis complete.\0".to_string(),
                "\0".to_string(),
                "This door will open with &push;&main-color=#D91818;Phazon Beam&pop;.\0"
                    .to_string(),
            ],
            _ => vec!["Task failed successfully\0".to_string()], // Vanilla doors do not need a scan point
        }
    }

    pub fn dependencies(&self) -> Vec<(u32, FourCC)> {
        // dependencies to add to the area

        let mut data: Vec<(u32, FourCC)> = Vec::new();

        let dep = (self.shield_cmdl().to_u32(), FourCC::from_bytes(b"CMDL"));
        if !data.contains(&dep) {
            data.push(dep);
        }

        let dep = (self.pattern0_txtr().to_u32(), FourCC::from_bytes(b"TXTR"));
        if !data.contains(&dep) {
            data.push(dep);
        }

        let dep = (self.pattern1_txtr().to_u32(), FourCC::from_bytes(b"TXTR"));
        if !data.contains(&dep) {
            data.push(dep);
        }

        let dep = (self.color_txtr().to_u32(), FourCC::from_bytes(b"TXTR"));
        if !data.contains(&dep) {
            data.push(dep);
        }

        let dep = (self.holorim_txtr().to_u32(), FourCC::from_bytes(b"TXTR"));
        if !data.contains(&dep) {
            data.push(dep);
        }

        let dep = (self.scan().to_u32(), FourCC::from_bytes(b"SCAN"));
        if !data.contains(&dep) {
            data.push(dep);
        }

        let dep = (self.strg().to_u32(), FourCC::from_bytes(b"STRG"));
        if !data.contains(&dep) {
            data.push(dep);
        }

        data.retain(|i| i.0 != 0xffffffff && i.0 != 0);

        data
    }

    pub fn iter() -> impl Iterator<Item = DoorType> {
        [
            DoorType::Blue,
            DoorType::PowerOnly,
            DoorType::Purple,
            DoorType::White,
            DoorType::Red,
            DoorType::Boost,
            DoorType::PowerBomb,
            DoorType::Bomb,
            DoorType::Boost,
            DoorType::Missile,
            DoorType::Charge,
            DoorType::Super,
            DoorType::Disabled,
            DoorType::Wavebuster,
            DoorType::Icespreader,
            DoorType::Flamethrower,
            DoorType::Ai,
            DoorType::Grapple,
            DoorType::Phazon,
            DoorType::Thermal,
            DoorType::XRay,
            DoorType::Scan,
            DoorType::VerticalBlue,
            DoorType::VerticalPowerOnly,
            DoorType::VerticalPurple,
            DoorType::VerticalWhite,
            DoorType::VerticalRed,
            DoorType::VerticalBoost,
            DoorType::VerticalPowerBomb,
            DoorType::VerticalBomb,
            DoorType::VerticalMissile,
            DoorType::VerticalCharge,
            DoorType::VerticalSuper,
            DoorType::VerticalDisabled,
            DoorType::VerticalWavebuster,
            DoorType::VerticalIcespreader,
            DoorType::VerticalFlamethrower,
            DoorType::VerticalAi,
            DoorType::VerticalGrapple,
            DoorType::VerticalPhazon,
            DoorType::VerticalThermal,
            DoorType::VerticalXRay,
            DoorType::VerticalScan,
        ]
        .iter()
        .copied()
    }

    pub fn vulnerability(&self) -> DamageVulnerability {
        const BASE_VULN: DamageVulnerability = DamageVulnerability {
            power: TypeVulnerability::Reflect as u32,
            ice: TypeVulnerability::Reflect as u32,
            wave: TypeVulnerability::Reflect as u32,
            plasma: TypeVulnerability::Reflect as u32,
            bomb: TypeVulnerability::Immune as u32,
            power_bomb: TypeVulnerability::Reflect as u32,
            missile: TypeVulnerability::Reflect as u32,
            boost_ball: TypeVulnerability::Immune as u32,
            phazon: TypeVulnerability::Immune as u32,

            enemy_weapon0: TypeVulnerability::Immune as u32,
            enemy_weapon1: TypeVulnerability::Immune as u32,
            enemy_weapon2: TypeVulnerability::Immune as u32,
            enemy_weapon3: TypeVulnerability::Immune as u32,

            unknown_weapon0: TypeVulnerability::Reflect as u32,
            unknown_weapon1: TypeVulnerability::Reflect as u32,
            unknown_weapon2: TypeVulnerability::Normal as u32,

            charged_beams: ChargedBeams {
                power: TypeVulnerability::Reflect as u32,
                ice: TypeVulnerability::Reflect as u32,
                wave: TypeVulnerability::Reflect as u32,
                plasma: TypeVulnerability::Reflect as u32,
                phazon: TypeVulnerability::Reflect as u32,
            },
            beam_combos: BeamCombos {
                power: TypeVulnerability::Reflect as u32,
                ice: TypeVulnerability::Reflect as u32,
                wave: TypeVulnerability::Reflect as u32,
                plasma: TypeVulnerability::Reflect as u32,
                phazon: TypeVulnerability::Reflect as u32,
            },
        };

        let door = self.to_horizontal();

        match door {
            DoorType::Blue => DamageVulnerability {
                power: TypeVulnerability::Normal as u32,
                ice: TypeVulnerability::Normal as u32,
                wave: TypeVulnerability::Normal as u32,
                plasma: TypeVulnerability::Normal as u32,
                bomb: TypeVulnerability::Normal as u32,
                power_bomb: TypeVulnerability::Normal as u32,
                missile: TypeVulnerability::Normal as u32,
                boost_ball: TypeVulnerability::Reflect as u32,
                phazon: TypeVulnerability::Normal as u32,

                enemy_weapon0: TypeVulnerability::Immune as u32,
                enemy_weapon1: TypeVulnerability::Immune as u32,
                enemy_weapon2: TypeVulnerability::Immune as u32,
                enemy_weapon3: TypeVulnerability::Immune as u32,

                unknown_weapon0: TypeVulnerability::Reflect as u32,
                unknown_weapon1: TypeVulnerability::Reflect as u32,
                unknown_weapon2: TypeVulnerability::Normal as u32,

                charged_beams: ChargedBeams {
                    power: TypeVulnerability::Normal as u32,
                    ice: TypeVulnerability::Normal as u32,
                    wave: TypeVulnerability::Normal as u32,
                    plasma: TypeVulnerability::Normal as u32,
                    phazon: TypeVulnerability::Normal as u32,
                },
                beam_combos: BeamCombos {
                    power: TypeVulnerability::Normal as u32,
                    ice: TypeVulnerability::Normal as u32,
                    wave: TypeVulnerability::Normal as u32,
                    plasma: TypeVulnerability::Normal as u32,
                    phazon: TypeVulnerability::Normal as u32,
                },
            },
            DoorType::Disabled => DamageVulnerability {
                power: TypeVulnerability::Immune as u32,
                ice: TypeVulnerability::Immune as u32,
                wave: TypeVulnerability::Immune as u32,
                plasma: TypeVulnerability::Immune as u32,
                bomb: TypeVulnerability::Immune as u32,
                power_bomb: TypeVulnerability::Immune as u32,
                missile: TypeVulnerability::Immune as u32,
                boost_ball: TypeVulnerability::Immune as u32,
                phazon: TypeVulnerability::Immune as u32,

                enemy_weapon0: TypeVulnerability::Immune as u32,
                enemy_weapon1: TypeVulnerability::Immune as u32,
                enemy_weapon2: TypeVulnerability::Immune as u32,
                enemy_weapon3: TypeVulnerability::Immune as u32,

                unknown_weapon0: TypeVulnerability::Reflect as u32,
                unknown_weapon1: TypeVulnerability::Reflect as u32,
                unknown_weapon2: TypeVulnerability::Normal as u32,

                charged_beams: ChargedBeams {
                    power: TypeVulnerability::Immune as u32,
                    ice: TypeVulnerability::Immune as u32,
                    wave: TypeVulnerability::Immune as u32,
                    plasma: TypeVulnerability::Immune as u32,
                    phazon: TypeVulnerability::Immune as u32,
                },
                beam_combos: BeamCombos {
                    power: TypeVulnerability::Immune as u32,
                    ice: TypeVulnerability::Immune as u32,
                    wave: TypeVulnerability::Immune as u32,
                    plasma: TypeVulnerability::Immune as u32,
                    phazon: TypeVulnerability::Immune as u32,
                },
            },
            DoorType::PowerOnly => {
                let mut vuln = BASE_VULN.clone();
                vuln.power = TypeVulnerability::Normal as u32;
                vuln.charged_beams.power = TypeVulnerability::Normal as u32;
                vuln.beam_combos.power = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Purple => {
                let mut vuln = BASE_VULN.clone();
                vuln.wave = TypeVulnerability::Normal as u32;
                vuln.charged_beams.wave = TypeVulnerability::Normal as u32;
                vuln.beam_combos.wave = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::White => {
                let mut vuln = BASE_VULN.clone();
                vuln.ice = TypeVulnerability::Normal as u32;
                vuln.charged_beams.ice = TypeVulnerability::Normal as u32;
                vuln.beam_combos.ice = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Red => {
                let mut vuln = BASE_VULN.clone();
                vuln.plasma = TypeVulnerability::Normal as u32;
                vuln.charged_beams.plasma = TypeVulnerability::Normal as u32;
                vuln.beam_combos.plasma = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::PowerBomb => {
                let mut vuln = BASE_VULN.clone();
                vuln.power_bomb = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Bomb => {
                let mut vuln = BASE_VULN.clone();
                vuln.bomb = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Boost => {
                let mut vuln = BASE_VULN.clone();
                vuln.boost_ball = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Missile => {
                let mut vuln = BASE_VULN.clone();
                vuln.missile = TypeVulnerability::Normal as u32;
                vuln.beam_combos.power = TypeVulnerability::Normal as u32;
                vuln.beam_combos.ice = TypeVulnerability::Normal as u32;
                vuln.beam_combos.wave = TypeVulnerability::Normal as u32;
                vuln.beam_combos.plasma = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Charge => {
                let mut vuln = BASE_VULN.clone();
                vuln.charged_beams.power = TypeVulnerability::Normal as u32;
                vuln.charged_beams.ice = TypeVulnerability::Normal as u32;
                vuln.charged_beams.wave = TypeVulnerability::Normal as u32;
                vuln.charged_beams.plasma = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Super => {
                let mut vuln = BASE_VULN.clone();
                vuln.beam_combos.power = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Wavebuster => {
                let mut vuln = BASE_VULN.clone();
                vuln.beam_combos.wave = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Icespreader => {
                let mut vuln = BASE_VULN.clone();
                vuln.beam_combos.ice = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Flamethrower => {
                let mut vuln = BASE_VULN.clone();
                vuln.beam_combos.plasma = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Ai => {
                let mut vuln = BASE_VULN.clone();
                vuln.enemy_weapon0 = TypeVulnerability::Normal as u32;
                vuln.enemy_weapon1 = TypeVulnerability::Normal as u32;
                vuln.enemy_weapon2 = TypeVulnerability::Normal as u32;
                vuln.enemy_weapon3 = TypeVulnerability::Normal as u32;
                vuln.unknown_weapon0 = TypeVulnerability::Normal as u32;
                vuln.unknown_weapon1 = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Phazon => {
                let mut vuln = BASE_VULN.clone();
                vuln.phazon = TypeVulnerability::Normal as u32;
                vuln.charged_beams.phazon = TypeVulnerability::Normal as u32;
                vuln.beam_combos.phazon = TypeVulnerability::Normal as u32;
                vuln
            }
            DoorType::Grapple => DoorType::Blue.vulnerability().clone(),
            DoorType::Thermal => DoorType::Blue.vulnerability().clone(),
            DoorType::XRay => DoorType::Blue.vulnerability().clone(),
            DoorType::Scan => DoorType::Blue.vulnerability().clone(),
            _ => panic!("Unhandled vulnerability for door {:?}", self),
        }
    }

    fn maybe_from_cmdl(cmdl: &u32) -> Option<Self> {
        match cmdl {
            0x0734977A => Some(DoorType::Blue),
            0xD5D49F54 => Some(DoorType::Blue), // Tallon Cargo Freight Lift
            0x33188D1B => Some(DoorType::Purple),
            0x59649E9D => Some(DoorType::White),
            0xBBBA1EC7 => Some(DoorType::Red),
            0x1E6337B6 => Some(DoorType::Red), // MQB (Save Station Door)
            0x18D0AEE6 => Some(DoorType::VerticalBlue),
            0x095B0B93 => Some(DoorType::VerticalPurple),
            0xB7A8A4C9 => Some(DoorType::VerticalWhite),
            _ => None,
        }
    }

    pub fn from_cmdl(cmdl: &u32) -> Self {
        Self::maybe_from_cmdl(cmdl)
            .unwrap_or_else(|| panic!("Unhandled cmdl id when derriving door type: 0x{:X}", cmdl))
    }

    pub fn is_door(cmdl: &u32) -> bool {
        if Self::maybe_from_cmdl(cmdl).is_some() {
            return true;
        }

        for door_type in Self::iter() {
            if door_type.shield_cmdl().to_u32() == *cmdl {
                return true;
            }
        }

        false
    }
}

impl BlastShieldType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(string: &str) -> Option<Self> {
        let test_str = string.trim().to_lowercase().replace([' ', '_', '-'], "");
        let test_str = test_str.as_str();

        match test_str {
            "missile" => Some(BlastShieldType::Missile),
            "missiles" => Some(BlastShieldType::Missile),
            "powerbomb" => Some(BlastShieldType::PowerBomb),
            "powerbombs" => Some(BlastShieldType::PowerBomb),
            "super" => Some(BlastShieldType::Super),
            "supermissile" => Some(BlastShieldType::Super),
            "supermissiles" => Some(BlastShieldType::Super),
            "wavebuster" => Some(BlastShieldType::Wavebuster),
            "icespreader" => Some(BlastShieldType::Icespreader),
            "flamethrower" => Some(BlastShieldType::Flamethrower),
            "none" => Some(BlastShieldType::None),
            "empty" => Some(BlastShieldType::None),
            "unchanged" => Some(BlastShieldType::Unchanged),
            "vanilla" => Some(BlastShieldType::Unchanged),
            "charge" => Some(BlastShieldType::Charge),
            "chargebeam" => Some(BlastShieldType::Charge),
            "grapple" => Some(BlastShieldType::Grapple),
            "grapplebeam" => Some(BlastShieldType::Grapple),
            "bomb" => Some(BlastShieldType::Bomb),
            "bombs" => Some(BlastShieldType::Bomb),
            "morphballbomb" => Some(BlastShieldType::Bomb),
            "morphballbombs" => Some(BlastShieldType::Bomb),
            "phazon" => Some(BlastShieldType::Phazon),
            "phazonbeam" => Some(BlastShieldType::Phazon),
            "thermal" => Some(BlastShieldType::Thermal),
            "thermalvisor" => Some(BlastShieldType::Thermal),
            "xray" => Some(BlastShieldType::XRay),
            "xrayvisor" => Some(BlastShieldType::XRay),
            "scan" => Some(BlastShieldType::Scan),
            "scanvisor" => Some(BlastShieldType::Scan),
            _ => None,
        }
    }

    pub const fn cmdl(&self) -> ResId<res_id::CMDL> {
        match self {
            BlastShieldType::PowerBomb => custom_asset_ids::POWER_BOMB_BLAST_SHIELD_CMDL,
            BlastShieldType::Super => custom_asset_ids::SUPER_MISSILE_BLAST_SHIELD_CMDL,
            BlastShieldType::Wavebuster => custom_asset_ids::WAVEBUSTER_BLAST_SHIELD_CMDL,
            BlastShieldType::Icespreader => custom_asset_ids::ICE_SPREADER_BLAST_SHIELD_CMDL,
            BlastShieldType::Flamethrower => custom_asset_ids::FLAMETHROWER_BLAST_SHIELD_CMDL,
            BlastShieldType::Charge => custom_asset_ids::CHARGE_BEAM_BLAST_SHIELD_CMDL,
            BlastShieldType::Grapple => custom_asset_ids::GRAPPLE_BEAM_BLAST_SHIELD_CMDL,
            BlastShieldType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_BLAST_SHIELD_CMDL,
            BlastShieldType::Phazon => custom_asset_ids::PHAZON_BEAM_BLAST_SHIELD_CMDL,
            BlastShieldType::Thermal => custom_asset_ids::THERMAL_VISOR_BLAST_SHIELD_CMDL,
            BlastShieldType::XRay => custom_asset_ids::XRAY_VISOR_BLAST_SHIELD_CMDL,
            BlastShieldType::Scan => custom_asset_ids::SCAN_VISOR_BLAST_SHIELD_CMDL,
            _ => ResId::new(0xEFDFFB8C), // Vanilla missile lock model
        }
    }

    pub const fn metal_body_txtr(&self) -> ResId<res_id::TXTR> {
        match self {
            BlastShieldType::PowerBomb => custom_asset_ids::POWER_BOMB_METAL_BODY_TXTR,
            BlastShieldType::Super => custom_asset_ids::SUPER_MISSILE_METAL_BODY_TXTR,
            BlastShieldType::Wavebuster => custom_asset_ids::WAVEBUSTER_METAL_BODY_TXTR,
            BlastShieldType::Icespreader => custom_asset_ids::ICE_SPREADER_METAL_BODY_TXTR,
            BlastShieldType::Flamethrower => custom_asset_ids::FLAMETHROWER_METAL_BODY_TXTR,
            BlastShieldType::Charge => custom_asset_ids::CHARGE_BEAM_METAL_BODY_TXTR,
            BlastShieldType::Grapple => custom_asset_ids::GRAPPLE_BEAM_METAL_BODY_TXTR,
            BlastShieldType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_METAL_BODY_TXTR,
            BlastShieldType::Phazon => custom_asset_ids::PHAZON_BEAM_METAL_BODY_TXTR,
            BlastShieldType::Thermal => custom_asset_ids::THERMAL_VISOR_METAL_BODY_TXTR,
            BlastShieldType::XRay => custom_asset_ids::XRAY_VISOR_METAL_BODY_TXTR,
            BlastShieldType::Scan => custom_asset_ids::SCAN_VISOR_METAL_BODY_TXTR,
            _ => ResId::new(0x6E09EA6B), // Vanilla missile lock txtr
        }
    }

    pub const fn glow_border_txtr(&self) -> ResId<res_id::TXTR> {
        match self {
            BlastShieldType::PowerBomb => custom_asset_ids::POWER_BOMB_GLOW_BORDER_TXTR,
            BlastShieldType::Super => custom_asset_ids::SUPER_MISSILE_GLOW_BORDER_TXTR,
            BlastShieldType::Wavebuster => custom_asset_ids::WAVEBUSTER_GLOW_BORDER_TXTR,
            BlastShieldType::Icespreader => custom_asset_ids::ICE_SPREADER_GLOW_BORDER_TXTR,
            BlastShieldType::Flamethrower => custom_asset_ids::FLAMETHROWER_GLOW_BORDER_TXTR,
            BlastShieldType::Charge => custom_asset_ids::CHARGE_BEAM_GLOW_BORDER_TXTR,
            BlastShieldType::Grapple => custom_asset_ids::GRAPPLE_BEAM_GLOW_BORDER_TXTR,
            BlastShieldType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_GLOW_BORDER_TXTR,
            BlastShieldType::Phazon => custom_asset_ids::PHAZON_BEAM_GLOW_BORDER_TXTR,
            BlastShieldType::Thermal => custom_asset_ids::THERMAL_VISOR_GLOW_BORDER_TXTR,
            BlastShieldType::XRay => custom_asset_ids::XRAY_VISOR_GLOW_BORDER_TXTR,
            BlastShieldType::Scan => custom_asset_ids::SCAN_VISOR_GLOW_BORDER_TXTR,
            _ => ResId::new(0x5B97098E), // Vanilla missile lock txtr
        }
    }

    pub const fn glow_trim_txtr(&self) -> ResId<res_id::TXTR> {
        match self {
            BlastShieldType::PowerBomb => custom_asset_ids::POWER_BOMB_GLOW_TRIM_TXTR,
            BlastShieldType::Super => custom_asset_ids::SUPER_MISSILE_GLOW_TRIM_TXTR,
            BlastShieldType::Wavebuster => custom_asset_ids::WAVEBUSTER_GLOW_TRIM_TXTR,
            BlastShieldType::Icespreader => custom_asset_ids::ICE_SPREADER_GLOW_TRIM_TXTR,
            BlastShieldType::Flamethrower => custom_asset_ids::FLAMETHROWER_GLOW_TRIM_TXTR,
            BlastShieldType::Charge => custom_asset_ids::CHARGE_BEAM_GLOW_TRIM_TXTR,
            BlastShieldType::Grapple => custom_asset_ids::GRAPPLE_BEAM_GLOW_TRIM_TXTR,
            BlastShieldType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_GLOW_TRIM_TXTR,
            BlastShieldType::Phazon => custom_asset_ids::PHAZON_BEAM_GLOW_TRIM_TXTR,
            BlastShieldType::Thermal => custom_asset_ids::THERMAL_VISOR_GLOW_TRIM_TXTR,
            BlastShieldType::XRay => custom_asset_ids::XRAY_VISOR_GLOW_TRIM_TXTR,
            BlastShieldType::Scan => custom_asset_ids::SCAN_VISOR_GLOW_TRIM_TXTR,
            _ => ResId::new(0x5C7B215C), // Vanilla missile lock txtr
        }
    }

    pub const fn animated_glow_txtr(&self) -> ResId<res_id::TXTR> {
        match self {
            BlastShieldType::PowerBomb => custom_asset_ids::POWER_BOMB_ANIMATED_GLOW_TXTR,
            BlastShieldType::Super => custom_asset_ids::SUPER_MISSILE_ANIMATED_GLOW_TXTR,
            BlastShieldType::Wavebuster => custom_asset_ids::WAVEBUSTER_ANIMATED_GLOW_TXTR,
            BlastShieldType::Icespreader => custom_asset_ids::ICE_SPREADER_ANIMATED_GLOW_TXTR,
            BlastShieldType::Flamethrower => custom_asset_ids::FLAMETHROWER_ANIMATED_GLOW_TXTR,
            BlastShieldType::Charge => custom_asset_ids::CHARGE_BEAM_ANIMATED_GLOW_TXTR,
            BlastShieldType::Grapple => custom_asset_ids::GRAPPLE_BEAM_ANIMATED_GLOW_TXTR,
            BlastShieldType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_ANIMATED_GLOW_TXTR,
            BlastShieldType::Phazon => custom_asset_ids::PHAZON_BEAM_ANIMATED_GLOW_TXTR,
            BlastShieldType::Thermal => custom_asset_ids::THERMAL_VISOR_ANIMATED_GLOW_TXTR,
            BlastShieldType::XRay => custom_asset_ids::XRAY_VISOR_ANIMATED_GLOW_TXTR,
            BlastShieldType::Scan => custom_asset_ids::SCAN_VISOR_ANIMATED_GLOW_TXTR,
            _ => ResId::new(0xFA0C2AE8), // Vanilla missile lock txtrw
        }
    }

    pub const fn metal_trim_txtr(&self) -> ResId<res_id::TXTR> {
        match self {
            BlastShieldType::PowerBomb => custom_asset_ids::POWER_BOMB_METAL_TRIM_TXTR,
            BlastShieldType::Super => custom_asset_ids::SUPER_MISSILE_METAL_TRIM_TXTR,
            BlastShieldType::Wavebuster => custom_asset_ids::WAVEBUSTER_METAL_TRIM_TXTR,
            BlastShieldType::Icespreader => custom_asset_ids::ICE_SPREADER_METAL_TRIM_TXTR,
            BlastShieldType::Flamethrower => custom_asset_ids::FLAMETHROWER_METAL_TRIM_TXTR,
            BlastShieldType::Charge => custom_asset_ids::CHARGE_BEAM_METAL_TRIM_TXTR,
            BlastShieldType::Grapple => custom_asset_ids::GRAPPLE_BEAM_METAL_TRIM_TXTR,
            BlastShieldType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_METAL_TRIM_TXTR,
            BlastShieldType::Phazon => custom_asset_ids::PHAZON_BEAM_METAL_TRIM_TXTR,
            BlastShieldType::Thermal => custom_asset_ids::THERMAL_VISOR_METAL_TRIM_TXTR,
            BlastShieldType::XRay => custom_asset_ids::XRAY_VISOR_METAL_TRIM_TXTR,
            BlastShieldType::Scan => custom_asset_ids::SCAN_VISOR_METAL_TRIM_TXTR,
            _ => ResId::new(0xFDE0023A), // Vanilla missile lock txtr
        }
    }

    pub const fn scan(&self) -> ResId<res_id::SCAN> {
        match self {
            BlastShieldType::PowerBomb => custom_asset_ids::POWER_BOMB_BLAST_SHIELD_SCAN,
            BlastShieldType::Super => custom_asset_ids::SUPER_MISSILE_BLAST_SHIELD_SCAN,
            BlastShieldType::Wavebuster => custom_asset_ids::WAVEBUSTER_BLAST_SHIELD_SCAN,
            BlastShieldType::Icespreader => custom_asset_ids::ICE_SPREADER_BLAST_SHIELD_SCAN,
            BlastShieldType::Flamethrower => custom_asset_ids::FLAMETHROWER_BLAST_SHIELD_SCAN,
            BlastShieldType::Charge => custom_asset_ids::CHARGE_BEAM_BLAST_SHIELD_SCAN,
            BlastShieldType::Grapple => custom_asset_ids::GRAPPLE_BEAM_BLAST_SHIELD_SCAN,
            BlastShieldType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_BLAST_SHIELD_SCAN,
            BlastShieldType::Phazon => custom_asset_ids::PHAZON_BEAM_BLAST_SHIELD_SCAN,
            BlastShieldType::Thermal => custom_asset_ids::THERMAL_VISOR_BLAST_SHIELD_SCAN,
            BlastShieldType::XRay => custom_asset_ids::XRAY_VISOR_BLAST_SHIELD_SCAN,
            BlastShieldType::Scan => ResId::invalid(), // scan actor added later
            BlastShieldType::Missile => ResId::<res_id::SCAN>::new(0x05F56F9D),
            _ => panic!("none/unchanged blast shield doesn't have scan"),
        }
    }

    pub const fn strg(&self) -> ResId<res_id::STRG> {
        match self {
            BlastShieldType::PowerBomb => custom_asset_ids::POWER_BOMB_BLAST_SHIELD_STRG,
            BlastShieldType::Super => custom_asset_ids::SUPER_MISSILE_BLAST_SHIELD_STRG,
            BlastShieldType::Wavebuster => custom_asset_ids::WAVEBUSTER_BLAST_SHIELD_STRG,
            BlastShieldType::Icespreader => custom_asset_ids::ICE_SPREADER_BLAST_SHIELD_STRG,
            BlastShieldType::Flamethrower => custom_asset_ids::FLAMETHROWER_BLAST_SHIELD_STRG,
            BlastShieldType::Charge => custom_asset_ids::CHARGE_BEAM_BLAST_SHIELD_STRG,
            BlastShieldType::Grapple => custom_asset_ids::GRAPPLE_BEAM_BLAST_SHIELD_STRG,
            BlastShieldType::Bomb => custom_asset_ids::MORPH_BALL_BOMBS_BLAST_SHIELD_STRG,
            BlastShieldType::Phazon => custom_asset_ids::PHAZON_BEAM_BLAST_SHIELD_STRG,
            BlastShieldType::Thermal => custom_asset_ids::THERMAL_VISOR_BLAST_SHIELD_STRG,
            BlastShieldType::XRay => custom_asset_ids::XRAY_VISOR_BLAST_SHIELD_STRG,
            BlastShieldType::Scan => ResId::invalid(), // scan actor added later
            BlastShieldType::Missile => ResId::<res_id::STRG>::new(0x265142BA),
            _ => panic!("none/unchanged blast shield doesn't have strg"),
        }
    }

    pub fn scan_text(&self) -> Vec<String> {
        match self {
            BlastShieldType::PowerBomb    => vec!["Analysis complete.\0".to_string(),
                                                  "\0".to_string(),
                                                  "There is an Advanced Blast Shield on the door blocking access. Analysis indicates that the Blast Shield is reinforced with &push;&main-color=#D91818;Bendezium&pop;, rendering it invulnerable to most weapons.\0".to_string(),
                                                 ],
            BlastShieldType::Super        => vec!["Analysis complete.\0".to_string(),
                                                  "\0".to_string(),
                                                  "There is an Advanced Blast Shield on the door blocking access. Analysis indicates that the Blast Shield is reinforced with &push;&main-color=#D91818;Cordite&pop;, rendering it invulnerable to most weapons.\0".to_string(),
                                                 ],
            BlastShieldType::Wavebuster   => vec!["Analysis complete.\0".to_string(),
                                                  "\0".to_string(),
                                                  "There is an Elemental Blast Shield on the door blocking access. Analysis indicates that the Blast Shield is invulnerable to standard Beam fire. Continuous exposure to &push;&main-color=#D91818;Extreme Amperage&pop; may damage it.\0".to_string(),
                                                 ],
            BlastShieldType::Icespreader  => vec!["Analysis complete.\0".to_string(),
                                                  "\0".to_string(),
                                                  "There is an Elemental Blast Shield on the door blocking access. Analysis indicates that the Blast Shield is invulnerable to standard Beam fire. A concussive blast augmented with &push;&main-color=#D91818;Extreme Cold&pop; may damage it.\0".to_string(),
                                                 ],
            BlastShieldType::Flamethrower => vec!["Analysis complete.\0".to_string(),
                                                  "\0".to_string(),
                                                  "There is an Elemental Blast Shield on the door blocking access. Analysis indicates that the Blast Shield is invulnerable to standard Beam fire. Continuous exposure to &push;&main-color=#D91818;Extreme Heat&pop; may damage it.\0".to_string(),
                                                 ],
            BlastShieldType::Charge       => vec!["Analysis complete.\0".to_string(),
                                                  "\0".to_string(),
                                                  "This Blast Shield can be destroyed with a &push;&main-color=#D91818;Concussive Blast&pop;.\0".to_string(),
                                                 ],
            BlastShieldType::Grapple      => vec!["Analysis complete.\0".to_string(),
                                                  "\0".to_string(),
                                                  "This Blast Shield can be removed with a &push;&main-color=#D91818;Strong Grip&pop;.\0".to_string(),
                                                 ],
            BlastShieldType::Bomb         => vec!["Analysis complete.\0".to_string(),
                                                  "\0".to_string(),
                                                  "There is a Blast Shield on the door blocking access. Analysis indicates that the Blast Shield is reinforced with &push;&main-color=#D91818;Sandstone&pop;, rendering it invulnerable to most weapons.\0".to_string(),
                                                 ],
            BlastShieldType::Phazon       => vec!["Analysis complete.\0".to_string(),
                                                  "\0".to_string(),
                                                  "This Blast Shield can be made to malfunction when exposed to &push;&main-color=#D91818;Extreme Radiation&pop;.\0".to_string(),
                                                 ],
            BlastShieldType::Thermal      => vec!["Analysis complete.\0".to_string(),
                                                  "\0".to_string(),
                                                  "This Blast Shield can be unlocked with &push;&main-color=#D91818;Thermal Visor&pop;.\0".to_string(),
                                                 ],
            BlastShieldType::XRay         => vec!["Analysis complete.\0".to_string(),
                                                  "\0".to_string(),
                                                  "This Blast Shield can be unlocked with &push;&main-color=#D91818;X-Ray Visor&pop;.\0".to_string(),
                                                 ],
            _ => vec!["\0".to_string()], // Vanilla missile locks do not have scans associated with the actor
        }
    }

    pub fn dependencies(&self, do_gibbs: bool) -> Vec<(u32, FourCC)> {
        // dependencies to add to the area

        let mut data: Vec<(u32, FourCC)> = vec![
            (self.cmdl().to_u32(), FourCC::from_bytes(b"CMDL")),
            (self.metal_body_txtr().to_u32(), FourCC::from_bytes(b"TXTR")),
            (
                self.glow_border_txtr().to_u32(),
                FourCC::from_bytes(b"TXTR"),
            ),
            (self.glow_trim_txtr().to_u32(), FourCC::from_bytes(b"TXTR")),
            (
                self.animated_glow_txtr().to_u32(),
                FourCC::from_bytes(b"TXTR"),
            ),
            (self.metal_trim_txtr().to_u32(), FourCC::from_bytes(b"TXTR")),
            (self.scan().to_u32(), FourCC::from_bytes(b"SCAN")),
            (self.strg().to_u32(), FourCC::from_bytes(b"STRG")),
        ];

        /* Gibbs */
        if do_gibbs {
            data.push((0xCDCBDF04, FourCC::from_bytes(b"PART")));

            data.push((0x185D5B02, FourCC::from_bytes(b"PART")));
            data.push((0x237B9BBB, FourCC::from_bytes(b"CMDL")));
            data.push((0x5C7B215C, FourCC::from_bytes(b"TXTR")));
            data.push((0xFDE0023A, FourCC::from_bytes(b"TXTR")));

            data.push((0x1D80CB59, FourCC::from_bytes(b"PART")));
            data.push((0x6FCB7BD5, FourCC::from_bytes(b"CMDL")));

            data.push((0x6FEBD6F7, FourCC::from_bytes(b"PART")));
            data.push((0x6BDD3EB9, FourCC::from_bytes(b"TXTR")));

            data.push((0x8F70D4F0, FourCC::from_bytes(b"PART")));
            data.push((0x8D680898, FourCC::from_bytes(b"CMDL")));

            data.push((0xA8842880, FourCC::from_bytes(b"PART")));
            data.push((0x6E84380A, FourCC::from_bytes(b"CMDL")));

            data.push((0xAEEDEF9D, FourCC::from_bytes(b"PART")));
            data.push((0xD73650EC, FourCC::from_bytes(b"CMDL")));
            data.push((0x6E09EA6B, FourCC::from_bytes(b"TXTR")));
            data.push((0x5B97098E, FourCC::from_bytes(b"TXTR")));
            data.push((0xFA0C2AE8, FourCC::from_bytes(b"TXTR")));

            data.push((0xD71C6D31, FourCC::from_bytes(b"PART")));
            data.push((0x0034CE07, FourCC::from_bytes(b"CMDL")));

            data.push((0xF0E89141, FourCC::from_bytes(b"PART")));
            data.push((0xC82B2BFE, FourCC::from_bytes(b"CMDL")));

            data.push((0xFAF20386, FourCC::from_bytes(b"PART")));
            data.push((0x4EBF5950, FourCC::from_bytes(b"CMDL")));
        }

        /* Sound */
        data.push((0x57FE7E67, FourCC::from_bytes(b"AGSC")));

        data.retain(|i| i.0 != 0xffffffff && i.0 != 0);
        data
    }

    pub fn iter() -> impl Iterator<Item = BlastShieldType> {
        [
            BlastShieldType::Missile,
            BlastShieldType::PowerBomb,
            BlastShieldType::Super,
            BlastShieldType::Wavebuster,
            BlastShieldType::Icespreader,
            BlastShieldType::Flamethrower,
            BlastShieldType::Charge,
            BlastShieldType::Grapple,
            BlastShieldType::Bomb,
            BlastShieldType::Phazon,
            BlastShieldType::Thermal,
            BlastShieldType::XRay,
            BlastShieldType::Scan,
        ]
        .iter()
        .copied()
    }

    pub fn vulnerability(&self) -> DamageVulnerability {
        self.door_type_counterpart().vulnerability()
    }

    pub const fn door_type_counterpart(&self) -> DoorType {
        match self {
            BlastShieldType::Missile => DoorType::Missile,
            BlastShieldType::PowerBomb => DoorType::PowerBomb,
            BlastShieldType::Super => DoorType::Super,
            BlastShieldType::Wavebuster => DoorType::Wavebuster,
            BlastShieldType::Icespreader => DoorType::Icespreader,
            BlastShieldType::Flamethrower => DoorType::Flamethrower,
            BlastShieldType::Charge => DoorType::Charge,
            BlastShieldType::Grapple => DoorType::Grapple,
            BlastShieldType::Bomb => DoorType::Bomb,
            BlastShieldType::Phazon => DoorType::Phazon,
            BlastShieldType::Thermal => DoorType::Thermal,
            BlastShieldType::XRay => DoorType::XRay,
            BlastShieldType::Scan => DoorType::Scan,
            _ => panic!("none/unchanged blast shield doesn't have door type counterpart"),
        }
    }
}
