use std::borrow::Cow;

use ppcasm::ppcasm;

use crate::custom_assets::custom_asset_ids;
use crate::dol_codegen::{
    branch_target, caves_for_version, rel_loader_selection, symbol_addr, symbol_addr_opt,
    TextEmitter,
};
use crate::dol_patcher::DolPatcher;
use crate::elevators::SpawnRoomData;
use crate::patch_config::{
    DifficultyBehavior, PatchConfig, PhazonDamageModifier, SuitDamageReduction, Version, Visor,
};
use crate::pickup_meta::PickupType;
use crate::txtr_conversions::{huerotate_color, huerotate_matrix};

fn patch_rel_loader(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let rel_loader = rel_loader_selection(version);
    let mut rel_loader_bytes = rel_loader.cave_bytes.to_vec();
    let padding = ((rel_loader_bytes.len() + 3) & !3) - rel_loader_bytes.len();
    rel_loader_bytes.extend([0u8; 4][..padding].iter().copied());
    let rel_loader_map = dol_linker::parse_symbol_table(
        "extra_assets/rel_loader_cave.bin.map".as_ref(),
        rel_loader.cave_map_str.lines().map(|l| Ok(l.to_owned())),
    )
    .map_err(|e| e.to_string())?;
    emitter.emit_at_cave_start(dol_patcher, rel_loader.cave_start, rel_loader_bytes)?;
    dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("PPCSetFpIEEEMode", version), {
        b { rel_loader_map["rel_loader_hook"] };
    }))?;
    Ok(())
}

fn patch_emit_is_memory_relay_active_func(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<u32, String> {
    let g_game_state = symbol_addr!("g_GameState", version);
    let state_for_world = symbol_addr!("StateForWorld__10CGameStateFUi", version);
    emitter.emit_addressed(dol_patcher, |addr| {
        ppcasm!(addr, {
            stwu      r1, -0x24(r1);
            mflr      r0;
            stw       r0, 0x24(r1);
            stw       r14, 0x20(r1);
            stw       r15, 0x1c(r1);
            stw       r29, 0x18(r1);
            mr        r29, r6;
            stw       r30, 0x14(r1);
            mr        r30, r4;
            stw       r31, 0x10(r1);
            mr        r31, r3;
            lis       r3, {g_game_state}@h;
            addi      r3, r3, {g_game_state}@l;
            lwz       r3, 0x0(r3);
            bl        {state_for_world};
            lwz       r14, 0x08(r3);
            lwz       r14, 0x00(r14);
            li        r0, 0;
            li        r3, 1;
            lwz       r6, 0x00(r14);
            addi      r6, r6, 1;
            cmpw      r3, r6;
            bge       {addr + 0x80};
            rlwinm    r3, r3, 2, 0, 29;
            lwzx      r15, r3, r14;
            rlwinm    r3, r3, 30, 4, 31;
            cmpw      r15, r31;
            bne       {addr + 0x78};
            li        r0, 1;
            b         {addr + 0x80};
            addi      r3, r3, 1;
            b         {addr + 0x54};
            mr        r3, r0;
            lwz       r0, 0x24(r1);
            lwz       r14, 0x20(r1);
            lwz       r15, 0x1c(r1);
            mr        r6, r29;
            lwz       r29, 0x18(r1);
            mr        r4, r30;
            lwz       r30, 0x14(r1);
            lwz       r31, 0x10(r1);
            mtlr      r0;
            addi      r1, r1, 0x24;
            blr;
        })
        .encoded_bytes()
    })
}

fn patch_set_pickup_icon_txtr(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    is_memory_relay_active_func: u32,
) -> Result<(), String> {
    let sitp_off: i32 = if version == Version::Pal {
        -0x5e3c
    } else if version == Version::NtscJ {
        -0x5e64
    } else {
        -0x5eb4
    };
    let draw_func = symbol_addr!("Draw__15CMappableObjectCFiRC13CMapWorldInfofb", version);
    let map_pickup_icon_txtr = custom_asset_ids::MAP_PICKUP_ICON_TXTR.to_u32();

    let sitp_addr = if version == Version::NtscJ || version == Version::Pal {
        let draw_func_284 = draw_func + 0x284;
        emitter.emit_addressed(dol_patcher, |addr| {
            ppcasm!(addr, {
                lwz          r3, 0x08(r18);
                lwz          r4, 0x6c(r1);
                lwz          r4, 0x24(r4);
                lbz          r0, 0x04(r4);
                cmpwi        r0, 1;
                beq          {addr + 0x20};
                lwz          r4, 0x08(r4);
                b            {addr + 0x24};
                lwz          r4, 0x0c(r4);
                bl           {is_memory_relay_active_func};
                lis          r31, {map_pickup_icon_txtr}@h;
                addi         r31, r31, {map_pickup_icon_txtr}@l;
                mr           r0, r31;
                cmpwi        r3, 0;
                lis          r31, 0xffff;
                ori          r31, r31, 0xffff;
                lwz          r3, {sitp_off}(r13);
                beq          {addr + 0x4c};
                fmr          f30, f14;
                b            {draw_func_284};
            })
            .encoded_bytes()
        })?
    } else {
        let draw_func_298 = draw_func + 0x298;
        emitter.emit_addressed(dol_patcher, |addr| {
            ppcasm!(addr, {
                lwz          r3, 0x08(r18);
                lwz          r4, 0x24(r31);
                lbz          r0, 0x04(r4);
                cmpwi        r0, 1;
                beq          {addr + 0x1c};
                lwz          r4, 0x08(r4);
                b            {addr + 0x20};
                lwz          r4, 0x0c(r4);
                bl           {is_memory_relay_active_func};
                cmpwi        r3, 0;
                lwz          r3, {sitp_off}(r13);
                lis          r6, {map_pickup_icon_txtr}@h;
                addi         r6, r6, {map_pickup_icon_txtr}@l;
                beq          {addr + 0x3c};
                fmr          f30, f14;
                b            {draw_func_298};
            })
            .encoded_bytes()
        })?
    };
    dol_patcher.ppcasm_patch(&ppcasm!(
        symbol_addr!("Case1B_Switch_Draw__CMappableObject", version) + ((structs::MapaObjectType::Pickup as u32) - 0x1b) * 4,
        { .long sitp_addr; }
    ))?;
    Ok(())
}

fn patch_warp_to_start(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let think_save_station = symbol_addr!(
        "ThinkSaveStation__22CScriptSpecialFunctionFfR13CStateManager",
        version
    );
    emitter.emit_and_patch(dol_patcher, think_save_station + 0x54, false, |addr| {
        ppcasm!(addr, {
            lis       r14, {symbol_addr!("g_Main", version)}@h;
            addi      r14, r14, {symbol_addr!("g_Main", version)}@l;
            lwz       r14, 0x0(r14);
            lwz       r14, 0x164(r14);
            lwz       r14, 0x34(r14);
            lbz       r0, 0x86(r14);
            cmpwi     r0, 0;
            beq       {addr + 0x34};
            lbz       r0, 0x89(r14);
            cmpwi     r0, 0;
            beq       {addr + 0x34};
            li        r4, 12;
            b         {addr + 0x38};
            li        r4, 9;
            andi      r14, r14, 0;
            b         {think_save_station + 0x58};
        })
        .encoded_bytes()
    })?;
    Ok(())
}

fn patch_spring_ball(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    config: &PatchConfig,
) -> Result<(), String> {
    let (
        velocity_offset,
        movement_state_offset,
        attached_actor_offset,
        energy_drain_offset,
        sb_out_of_water_ticks_offset,
        surface_restraint_type_offset,
        morph_ball_offset,
    ) = if version == Version::NtscU0_00
        || version == Version::NtscU0_01
        || version == Version::NtscK
    {
        (0x138, 0x258, 0x26c, 0x274, 0x2b0, 0x2ac, 0x768)
    } else {
        (0x148, 0x268, 0x27c, 0x284, 0x2c0, 0x2bc, 0x778)
    };
    let spring_ball_item_kind = if config.spring_ball_item != PickupType::Nothing {
        Some(config.spring_ball_item.kind())
    } else {
        None
    };
    let has_power_up_sym = symbol_addr!(
        "HasPowerUp__12CPlayerStateCFQ212CPlayerState9EItemType",
        version
    );
    let is_movement_allowed_sym = symbol_addr!("IsMovementAllowed__10CMorphBallCFv", version);
    let bomb_jump_sym = symbol_addr!("BombJump__7CPlayerFRC9CVector3fR13CStateManager", version);
    let set_move_state_sym = symbol_addr!(
        "SetMoveState__7CPlayerFQ27NPlayer20EPlayerMovementStateR13CStateManager",
        version
    );
    let compute_boost_ball_sym = symbol_addr!(
        "ComputeBoostBallMovement__10CMorphBallFRC11CFinalInputRC13CStateManagerf",
        version
    );
    let get_energy_drain_sym =
        symbol_addr!("GetEnergyDrainIntensity__18CPlayerEnergyDrainCFv", version);

    // All three spring-ball parts are one contiguous allocation (the +-32 KB BC branches
    // between them require contiguity).
    let compute_spring_ball_movement = emitter.emit_addressed(dol_patcher, |base| {
        let data_addr = base + 0x1b4;
        let sb_start = ppcasm!(base, {
            stwu      r1, -0x20(r1);
            mflr      r0;
            stw       r0, 0x20(r1);
            fmr       f15, f1;
            stw       r31, 0x1c(r1);
            stw       r30, 0x18(r1);
            mr        r30, r5;
            stw       r29, 0x14(r1);
            mr        r29, r4;
            stw       r28, 0x10(r1);
            mr        r28, r3;
            lwz       r14, 0x84c(r30);
            lwz       r15, 0x8b8(r30);
            lis       r16, {data_addr}@h;
            addi      r16, r16, {data_addr}@l;
            lwz       r17, {morph_ball_offset}(r14);
            lfs       f1, 0x40(r14);
            stfs      f1, 0x00(r16);
            lfs       f1, 0x50(r14);
            stfs      f1, 0x04(r16);
            lfs       f1, 0x60(r14);
            stfs      f1, 0x08(r16);
            lwz       r0, 0x0c(r16);
            cmplwi    r0, 0;
            bgt       {base + 0x14c};
            lwz       r0, {movement_state_offset}(r14);
            cmplwi    r0, 0;
            beq       {base + 0x84};
            b         {base + 0x14c};
            cmplwi    r0, 4;
            bne       {base + 0x14c};
            lwz       r0, {sb_out_of_water_ticks_offset}(r14);
            cmplwi    r0, 2;
            bne       {base + 0x90};
            lwz       r0, {surface_restraint_type_offset}(r14);
            b         {base + 0x94};
            li        r0, 4;
            cmplwi    r0, 7;
            beq       {base + 0x14c};
            mr        r3, r28;
            bl        {is_movement_allowed_sym};
            cmplwi    r3, 0;
            beq       {base + 0x14c};
        })
        .encoded_bytes();
        let sb_item_addr = base + sb_start.len() as u32;
        let sb_item = if let Some(kind) = spring_ball_item_kind {
            ppcasm!(sb_item_addr, {
                lwz       r3, 0x0(r15);
                li        r4, {kind};
                bl        {has_power_up_sym};
                cmplwi    r3, 0;
                beq       {base + 0x14c};
            })
            .encoded_bytes()
        } else {
            ppcasm!(sb_item_addr, {
                nop;
                nop;
                nop;
                nop;
                nop;
            })
            .encoded_bytes()
        };
        let sb_end_addr = sb_item_addr + sb_item.len() as u32;
        let sb_end = ppcasm!(sb_end_addr, {
            lhz       r0, {attached_actor_offset}(r14);
            cmplwi    r0, 65535;
            bne       {base + 0x14c};
            addi      r3, r14, {energy_drain_offset};
            bl        {get_energy_drain_sym};
            fcmpu     cr0, f1, f14;
            bgt       {base + 0x14c};
            lwz       r0, 0x187c(r28);
            cmplwi    r0, 0;
            bne       {base + 0x14c};
            lfs       f1, 0x14(r29);
            fcmpu     cr0, f1, f14;
            ble       {base + 0x14c};
            lfs       f16, {velocity_offset}(r14);
            lfs       f17, {velocity_offset + 4}(r14);
            mr        r3, r14;
            mr        r4, r16;
            mr        r5, r30;
            bl        {bomb_jump_sym};
            stfs      f16, {velocity_offset}(r14);
            stfs      f17, {velocity_offset + 4}(r14);
            lfs       f17, 0x1dfc(r17);
            fcmpu     cr0, f17, f14;
            ble       {base + 0x130};
            lfs       f17, 0x10(r16);
            lfs       f16, {velocity_offset + 8}(r14);
            fdivs     f16, f16, f17;
            stfs      f16, {velocity_offset + 8}(r14);
            mr        r3, r14;
            li        r4, 4;
            mr        r5, r29;
            bl        {set_move_state_sym};
            li        r3, 40;
            stw       r3, 0x0c(r16);
            b         {base + 0x160};
            lwz       r3, 0x0c(r16);
            cmplwi    r3, 0;
            beq       {base + 0x160};
            addi      r3, r3, -1;
            stw       r3, 0x0c(r16);
            mr        r3, r28;
            mr        r4, r29;
            mr        r5, r30;
            fmr       f1, f15;
            bl        {compute_boost_ball_sym};
            andi      r14, r14, 0;
            andi      r15, r15, 0;
            andi      r16, r16, 0;
            andi      r17, r17, 0;
            lwz       r0, 0x20(r1);
            fmr       f1, f15;
            fmr       f15, f14;
            fmr       f16, f14;
            fmr       f17, f14;
            lwz       r31, 0x1c(r1);
            lwz       r30, 0x18(r1);
            lwz       r29, 0x14(r1);
            lwz       r28, 0x10(r1);
            mtlr      r0;
            addi      r1, r1, 0x20;
            blr;
        data:
            .float 0.0;
            .float 0.0;
            .float 0.0;
            .long 0;
            .float 1.5;
        })
        .encoded_bytes();
        let mut all = sb_start;
        all.extend(sb_item);
        all.extend(sb_end);
        all
    })?;
    #[rustfmt::skip]
    dol_patcher.ppcasm_patch(&ppcasm!(
        symbol_addr!("ComputeBallMovement__10CMorphBallFRC11CFinalInputR13CStateManagerf", version) + 0x2c,
        { bl {compute_spring_ball_movement}; }))?;

    let spring_ball_cooldown = compute_spring_ball_movement + 0x1c0;

    let (call_leave_morph_ball_offset, call_enter_morph_ball_offset) =
        if version == Version::NtscJ || version == Version::Pal {
            (0x850, 0x940)
        } else {
            (0xa34, 0xb24)
        };
    let update_morph_ball_transition = symbol_addr!(
        "UpdateMorphBallTransition__7CPlayerFfR13CStateManager",
        version
    );
    let leave_morph_ball_sym =
        symbol_addr!("LeaveMorphBallState__7CPlayerFR13CStateManager", version);
    let enter_morph_ball_sym =
        symbol_addr!("EnterMorphBallState__7CPlayerFR13CStateManager", version);

    let sb_unmorph_addr = emitter.emit_addressed(dol_patcher, |addr| {
        ppcasm!(addr, {
            stwu      r1, -0x18(r1);
            mflr      r0;
            stw       r0, 0x18(r1);
            fmr       f15, f1;
            stw       r31, 0x10(r1);
            mr        r31, r3;
            stw       r30, 0x14(r1);
            mr        r30, r4;
            lis       r14, {spring_ball_cooldown}@h;
            addi      r14, r14, {spring_ball_cooldown}@l;
            li        r0, 0;
            stw       r0, 0x0(r14);
            mr        r3, r31;
            mr        r4, r30;
            bl        {leave_morph_ball_sym};
            andi      r14, r14, 0;
            lwz       r0, 0x18(r1);
            lwz       r31, 0x14(r1);
            lwz       r30, 0x10(r1);
            mtlr      r0;
            addi      r1, r1, 0x18;
            blr;
        })
        .encoded_bytes()
    })?;
    dol_patcher.ppcasm_patch(&ppcasm!(
        update_morph_ball_transition + call_leave_morph_ball_offset,
        {
            bl { sb_unmorph_addr };
        }
    ))?;

    let sb_morph_addr = emitter.emit_addressed(dol_patcher, |addr| {
        ppcasm!(addr, {
            stwu      r1, -0x18(r1);
            mflr      r0;
            stw       r0, 0x18(r1);
            fmr       f15, f1;
            stw       r31, 0x10(r1);
            mr        r31, r3;
            stw       r30, 0x14(r1);
            mr        r30, r4;
            lis       r14, {spring_ball_cooldown}@h;
            addi      r14, r14, {spring_ball_cooldown}@l;
            li        r0, 0;
            stw       r0, 0x0(r14);
            mr        r3, r31;
            mr        r4, r30;
            bl        {enter_morph_ball_sym};
            andi      r14, r14, 0;
            lwz       r0, 0x18(r1);
            lwz       r31, 0x14(r1);
            lwz       r30, 0x10(r1);
            mtlr      r0;
            addi      r1, r1, 0x18;
            blr;
        })
        .encoded_bytes()
    })?;
    dol_patcher.ppcasm_patch(&ppcasm!(
        update_morph_ball_transition + call_enter_morph_ball_offset,
        {
            bl { sb_morph_addr };
        }
    ))?;

    Ok(())
}

fn patch_custom_items(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let first_custom_item_idx = -((PickupType::ArtifactOfNewborn.kind() + 1) as i32);
    let (actor_flags_offset, out_of_water_ticks_offset, fluid_depth_offset) =
        if [Version::Pal, Version::NtscJ, Version::NtscU0_02].contains(&version) {
            (0xf0, 0x2c0, 0x838)
        } else {
            (0xe4, 0x2b0, 0x828)
        };
    let (probability_offset, life_time_offset) =
        if [Version::Pal, Version::NtscJ].contains(&version) {
            (0x274, 0x27c)
        } else {
            (0x264, 0x26c)
        };
    let init_power_up_sym = symbol_addr!(
        "InitializePowerUp__12CPlayerStateFQ212CPlayerState9EItemTypei",
        version
    );
    let power_up_max_values_sym = symbol_addr!("CPlayerState_PowerUpMaxValues", version);
    let freeze_sym = symbol_addr!("Freeze__7CPlayerFR13CStateManagerUiUsUi", version);

    let has_power_up_sym = symbol_addr!(
        "HasPowerUp__12CPlayerStateCFQ212CPlayerState9EItemType",
        version
    );
    let get_item_amount_sym = symbol_addr!(
        "GetItemAmount__12CPlayerStateCFQ212CPlayerState9EItemType",
        version
    );
    let get_item_capacity_sym = symbol_addr!(
        "GetItemCapacity__12CPlayerStateCFQ212CPlayerState9EItemType",
        version
    );
    let decr_pickup_sym = symbol_addr!(
        "DecrPickUp__12CPlayerStateFQ212CPlayerState9EItemTypei",
        version
    );

    // ci_* stubs are overflow-eligible: make_pic relocates their external branches and stub-local
    // data references (e.g. the lis/addi to r3_backup and the .float below).
    emitter.emit_and_patch(dol_patcher, init_power_up_sym + 0x1c, false, |addr| {
        ppcasm!(addr, {
            mr           r29, r4;
            mr           r14, r5;
            lis          r15, {power_up_max_values_sym}@h;
            addi         r15, r15, {power_up_max_values_sym}@l;
            lwz          r4, 0x14(r1);
            lwz          r3, {life_time_offset}(r4);
            cmpwi        r3, 0;
            lhz          r3, {probability_offset}(r4);
            bne          check_custom_item;
            cmpwi        r3, 0x42c8;
            bne          check_custom_item;
            li           r3, {PickupType::PowerSuit.kind()};
            rlwinm       r0, r3, 0x3, 0x0, 0x1c;
            add          r3, r31, r0;
            addi         r3, r3, 0x28;
            lwz          r4, 0x4(r3);
            addi         r4, r4, 1;
            stw          r4, 0x4(r3);
        check_custom_item:
            cmpwi        r29, {PickupType::ArtifactOfNewborn.kind()};
            ble          continue_init_power_up;
            cmpwi        r29, {PickupType::Nothing.kind()};
            bge          check_missile_launcher;
            li           r3, {PickupType::UnknownItem2.kind()};
            rlwinm       r0, r3, 0x3, 0x0, 0x1c;
            add          r3, r31, r0;
            addi         r3, r3, 0x2c;
            li           r4, {first_custom_item_idx};
            add          r4, r4, r29;
            li           r0, 1;
            slw          r0, r4, r4;
            lwz          r0, 0x0(r3);
            cmpwi        r14, 0;
            blt          remove_custom_item;
            or           r0, r4, r4;
            b            set_custom_item;
        remove_custom_item:
            not          r4, r4;
            and          r0, r4, r4;
        set_custom_item:
            stw          r4, 0x0(r3);
        check_missile_launcher:
            cmpwi        r29, {PickupType::MissileLauncher.kind()};
            bne          check_power_bomb;
            li           r3, {PickupType::Missile.kind()};
            lwz          r0, {PickupType::Missile.kind() * 4}(r15);
            b            incr_capacity;
        check_power_bomb:
            cmpwi        r29, {PickupType::PowerBombLauncher.kind()};
            bne          check_ice_trap;
            li           r3, {PickupType::PowerBomb.kind()};
            lwz          r0, {PickupType::PowerBomb.kind() * 4}(r15);
            b            incr_capacity;
        check_ice_trap:
            cmpwi        r29, {PickupType::IceTrap.kind()};
            bne          check_floaty_jump;
            mr           r16, r5;
            lwz          r3, 0x84c(r30);
            mr           r4, r25;
            lis          r5, 0x6FC0;
            ori          r5, r5, 0x3D46;
            li           r6, 0xC34;
            lis          r7, 0x2B75;
            ori          r7, r7, 0x7945;
            bl           {freeze_sym};
            lis          r5, data@h;
            addi         r5, r5, data@l;
            lfs          f14, 0x0(r5);
            lwz          r5, 0x8b8(r30);
            lwz          r5, 0x0(r5);
            lfs          f15, 0x0c(r5);
            fsubs        f15, f15, f14;
            stfs         f15, 0x0c(r5);
            fcmpu        cr0, f15, f28;
            bgt          not_dead_from_ice_trap;
            lwz          r4, 0x0(r5);
            andis        r4, r4, 0x7fff;
            stw          r4, 0x0(r5);
        not_dead_from_ice_trap:
            b            end_init_power_up;
        check_floaty_jump:
            cmpwi        r29, {PickupType::FloatyJump.kind()};
            bne          continue_init_power_up;
            lwz          r3, 0x84c(r30);
            lwz          r0, {out_of_water_ticks_offset}(r3);
            lwz          r5, {actor_flags_offset}(r3);
            mr           r4, r5;
            srwi         r5, r5, 14;
            andi         r5, r5, 7;
            lis          r6, 0xffff;
            ori          r6, r6, 0x3fff;
            and          r4, r4, r6;
            cmpwi        r14, 0;
            blt          remove_floaty_jump;
            addi         r5, r5, 1;
            andi         r5, r5, 7;
            slwi         r5, r5, 14;
            or           r4, r4, r5;
            cmpwi        r0, 2;
            bne          apply_underwater_floaty_jump;
            lis          r5, 0x41a0;
            b            apply_floaty_jump;
        remove_floaty_jump:
            cmpwi        r0, 2;
            bne          do_not_decrement_fluid_count_more_than_one;
            cmpwi        r5, 0;
            ble          do_not_decrement_fluid_count;
            b            decrement_fluid_count;
        do_not_decrement_fluid_count_more_than_one:
            cmpwi        r5, 1;
            ble          do_not_decrement_fluid_count;
        decrement_fluid_count:
            addi         r5, r5, -1;
        do_not_decrement_fluid_count:
            andi         r5, r5, 7;
            slwi         r5, r5, 14;
            or           r4, r4, r5;
            cmpwi        r0, 2;
            bne          apply_underwater_floaty_jump;
            lis          r5, 0;
        apply_floaty_jump:
            stw          r5, {fluid_depth_offset}(r3);
        apply_underwater_floaty_jump:
            stw          r4, {actor_flags_offset}(r3);
            b            end_init_power_up;
        incr_capacity:
            rlwinm       r0, r3, 0x3, 0x0, 0x1c;
            add          r3, r31, r0;
            addi         r3, r3, 0x28;
            lwz          r4, 0x4(r3);
            add          r4, r4, r14;
            cmpw         r4, r0;
            ble          incr_capacity_check_for_negative;
            mr           r4, r0;
            b            incr_capacity_set_capacity;
        incr_capacity_check_for_negative:
            cmpwi        r4, 0;
            bge          incr_capacity_set_capacity;
            li           r4, 0;
        incr_capacity_set_capacity:
            stw          r4, 0x4(r3);
            lwz          r4, 0x0(r3);
            add          r4, r4, r14;
            lwz          r0, 0x4(r3);
            cmpw         r4, r0;
            ble          incr_amount_check_for_negative;
            mr           r4, r0;
            b            incr_amount_set_amount;
        incr_amount_check_for_negative:
            cmpwi        r4, 0;
            bge          incr_amount_set_amount;
            li           r4, 0;
        incr_amount_set_amount:
            stw          r4, 0x0(r3);
        end_init_power_up:
            mr           r5, r14;
            andi         r14, r14, 0;
            andi         r15, r15, 0;
            andi         r16, r16, 0;
            fmr          f14, f28;
            fmr          f15, f28;
            b            {init_power_up_sym + 0x108};
        continue_init_power_up:
            mr           r5, r14;
            andi         r14, r14, 0;
            andi         r15, r15, 0;
            andi         r16, r16, 0;
            fmr          f14, f28;
            fmr          f15, f28;
            cmpwi        r29, 0;
            b            {init_power_up_sym + 0x20};
        data:
            .float    75.0;
        })
        .encoded_bytes()
    })?;

    emitter.emit_and_patch(dol_patcher, has_power_up_sym, false, |addr| {
        ppcasm!(addr, {
            lis          r5, r3_backup@h;
            addi         r5, r5, r3_backup@l;
            stw          r3, 0x0(r5);
            stw          r4, 0x4(r5);
            cmpwi        r4, {PickupType::ArtifactOfNewborn.kind()};
            ble          not_custom_item;
            li           r4, {PickupType::UnknownItem2.kind()};
            rlwinm       r0, r4, 0x3, 0x0, 0x1c;
            add          r4, r3, r0;
            addi         r4, r4, 0x2c;
            lwz          r0, 0x0(r4);
            lwz          r4, 0x4(r5);
            li           r3, {first_custom_item_idx};
            add          r3, r3, r4;
            srw          r0, r3, r3;
            andi         r3, r3, 1;
        powerup_not_valid:
            blr;
        not_custom_item:
            lwz          r4, 0x4(r5);
            cmpwi        r4, 0;
            blt          powerup_not_valid;
            b            {has_power_up_sym + 0x8};
        r3_backup:
            .long 0;
        r4_backup:
            .long 0;
        })
        .encoded_bytes()
    })?;

    emitter.emit_and_patch(dol_patcher, get_item_amount_sym, false, |addr| {
        ppcasm!(addr, {
            lis          r5, r3_backup@h;
            addi         r5, r5, r3_backup@l;
            stw          r3, 0x0(r5);
            stw          r4, 0x4(r5);
            mr           r4, r3;
            li           r3, {PickupType::UnknownItem2.kind()};
            rlwinm       r3, r3, 0x3, 0x0, 0x1c;
            add          r3, r4, r3;
            addi         r3, r3, 0x2c;
            lwz          r3, 0x0(r3);
            mr           r0, r3;
            lwz          r4, 0x4(r5);
            cmpwi        r4, {PickupType::Missile.kind()};
            bne          check_power_bomb;
            andi         r0, r3, {PickupType::MissileLauncher.custom_item_value()};
            cmpwi        r3, 0;
            beq          no_launcher;
            lwz          r4, 0x0(r5);
            li           r3, {PickupType::Missile.kind()};
            rlwinm       r3, r3, 0x3, 0x0, 0x1c;
            add          r3, r4, r3;
            addi         r3, r3, 0x2c;
            lwz          r3, 0x0(r3);
            cmpwi        r3, 0;
            ble          no_launcher;
            andi         r0, r3, {PickupType::UnlimitedMissiles.custom_item_value()};
            cmpwi        r3, 0;
            beq          not_unlimited_or_not_pb_missiles;
            li           r3, 255;
            b            is_unlimited;
        check_power_bomb:
            lwz          r4, 0x4(r5);
            cmpwi        r4, {PickupType::PowerBomb.kind()};
            bne          not_unlimited_or_not_pb_missiles;
            andi         r0, r3, {PickupType::PowerBombLauncher.custom_item_value()};
            cmpwi        r3, 0;
            beq          no_launcher;
            lwz          r4, 0x0(r5);
            li           r3, {PickupType::PowerBomb.kind()};
            rlwinm       r3, r3, 0x3, 0x0, 0x1c;
            add          r3, r4, r3;
            addi         r3, r3, 0x2c;
            lwz          r3, 0x0(r3);
            cmpwi        r3, 0;
            ble          no_launcher;
            andi         r0, r3, {PickupType::UnlimitedPowerBombs.custom_item_value()};
            cmpwi        r3, 0;
            beq          not_unlimited_or_not_pb_missiles;
            li           r3, 8;
            b            is_unlimited;
        no_launcher:
            li           r3, 0;
        is_unlimited:
            lwz          r4, 0x4(r5);
            blr;
        not_unlimited_or_not_pb_missiles:
            lwz          r3, 0x0(r5);
            lwz          r4, 0x4(r5);
            cmpwi        r4, 0;
            blt          item_type_negative;
            b            {get_item_amount_sym + 0x8};
        item_type_negative:
            li           r3, 0;
            blr;
        r3_backup:
            .long 0;
        r4_backup:
            .long 0;
        })
        .encoded_bytes()
    })?;

    emitter.emit_and_patch(dol_patcher, get_item_capacity_sym, false, |addr| {
        // r6/r7 are free volatiles (GetItemCapacity takes only r3, r4).
        ppcasm!(addr, {
            mr           r6, r3;
            mr           r7, r4;
            li           r4, {PickupType::UnknownItem2.kind()};
            rlwinm       r0, r4, 0x3, 0x0, 0x1c;
            add          r4, r3, r0;
            addi         r4, r4, 0x2c;
            lwz          r0, 0x0(r4);
            mr           r4, r7;
            cmpwi        r4, {PickupType::Missile.kind()};
            bne          check_power_bomb;
            andi         r0, r3, {PickupType::MissileLauncher.custom_item_value()};
            cmpwi        r3, 0;
            beq          no_launcher;
            mr           r3, r6;
            li           r4, {PickupType::Missile.kind()};
            rlwinm       r4, r4, 0x3, 0x0, 0x1c;
            add          r3, r3, r4;
            addi         r3, r3, 0x2c;
            lwz          r3, 0x0(r3);
            cmpwi        r3, 0;
            ble          no_launcher;
            andi         r0, r3, {PickupType::UnlimitedMissiles.custom_item_value()};
            cmpwi        r3, 0;
            beq          not_unlimited_or_not_pb_missiles;
            li           r3, 255;
            b            custom_capacity_returned;
        check_power_bomb:
            cmpwi        r4, {PickupType::PowerBomb.kind()};
            bne          not_unlimited_or_not_pb_missiles;
            andi         r0, r3, {PickupType::PowerBombLauncher.custom_item_value()};
            cmpwi        r3, 0;
            beq          no_launcher;
            mr           r3, r6;
            li           r4, {PickupType::PowerBomb.kind()};
            rlwinm       r4, r4, 0x3, 0x0, 0x1c;
            add          r3, r3, r4;
            addi         r3, r3, 0x2c;
            lwz          r3, 0x0(r3);
            cmpwi        r3, 0;
            ble          no_launcher;
            andi         r0, r3, {PickupType::UnlimitedPowerBombs.custom_item_value()};
            cmpwi        r3, 0;
            beq          not_unlimited_or_not_pb_missiles;
            li           r3, 8;
            b            custom_capacity_returned;
        no_launcher:
            li           r3, 0;
        custom_capacity_returned:
            mr           r4, r7;
        powerup_not_valid:
            blr;
        not_unlimited_or_not_pb_missiles:
            mr           r3, r6;
            mr           r4, r7;
            cmpwi        r4, 0;
            blt          powerup_not_valid;
            b            {get_item_capacity_sym + 0x8};
        })
        .encoded_bytes()
    })?;

    emitter.emit_and_patch(dol_patcher, decr_pickup_sym, false, |addr| {
        // r5 is the third argument (amount), so use r7 as the backup base pointer.
        ppcasm!(addr, {
            lis          r7, r3_backup@h;
            addi         r7, r7, r3_backup@l;
            stw          r3, 0x0(r7);
            stw          r4, 0x4(r7);
            li           r4, {PickupType::UnknownItem2.kind()};
            rlwinm       r0, r4, 0x3, 0x0, 0x1c;
            add          r4, r3, r0;
            addi         r4, r4, 0x28;
            lwz          r0, 0x0(r4);
            lwz          r4, 0x4(r7);
            cmpwi        r4, {PickupType::Missile.kind()};
            bne          check_power_bomb;
            andi         r0, r3, {PickupType::UnlimitedMissiles.custom_item_value()};
            cmpwi        r3, 0;
            beq          pre_cleanup;
            li           r0, 1;
            b            cleanup;
        check_power_bomb:
            cmpwi        r4, {PickupType::PowerBomb.kind()};
            bne          cleanup;
            andi         r0, r3, {PickupType::UnlimitedPowerBombs.custom_item_value()};
            cmpwi        r3, 0;
            beq          pre_cleanup;
            li           r0, 1;
            b            cleanup;
        pre_cleanup:
            li           r0, 0;
        cleanup:
            lwz          r3, 0x0(r7);
            lwz          r4, 0x4(r7);
            cmpwi        r0, 0;
            beq          not_unlimited;
        powerup_not_valid:
            blr;
        not_unlimited:
            cmpwi        r4, 0;
            blt          powerup_not_valid;
            b            {decr_pickup_sym + 0x8};
        r3_backup:
            .long 0;
        r4_backup:
            .long 0;
        })
        .encoded_bytes()
    })?;

    Ok(())
}

fn patch_restore_ntsc_00(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    config: &PatchConfig,
) -> Result<(), String> {
    if [Version::Pal, Version::NtscJ].contains(&version) {
        let cridley_addr = symbol_addr!(
            "AcceptScriptMsg__7CRidleyF20EScriptObjectMessage9TUniqueIdR13CStateManager",
            version
        );
        dol_patcher.ppcasm_patch(&ppcasm!(cridley_addr + 0x830, {
            nop;
        }))?;
        dol_patcher.ppcasm_patch(&ppcasm!(cridley_addr + 0x840, {
            nop;
        }))?;
        let restore_addr = emitter.emit_addressed(dol_patcher, |addr| {
            ppcasm!(addr, {
                lbz       r0, 0x0140(r3);
                rlwinm.   r0, r0, 26, 31, 31;
                bne       {addr + 0x18};
                lwz       r0, 0x13c(r3);
                cmpwi     r0, 6;
                bne       {addr + 0x20};
                fmr       f0, f14;
                stfs      f0, 0xad0(r30);
                b         {cridley_addr + 0x898};
            })
            .encoded_bytes()
        })?;
        dol_patcher.ppcasm_patch(&ppcasm!(cridley_addr + 0x884, {
            beq { cridley_addr + 0x88C };
            b   { restore_addr + 0x18 };
            b   { restore_addr };
            nop;
            nop;
        }))?;
    }

    if version == Version::NtscU0_02 || version == Version::Pal || version == Version::NtscJ {
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("SidewaysDashAllowed__7CPlayerCFffRC11CFinalInputR13CStateManager", version) + 0x3c, {
            b { symbol_addr!("SidewaysDashAllowed__7CPlayerCFffRC11CFinalInputR13CStateManager", version) + 0x54 };
        }))?;
    }

    dol_patcher.ppcasm_patch(
        &ppcasm!(symbol_addr!("g_maxPhazonLagBeforeDamaging", version), {
            .float 0.2;
        }),
    )?;

    if config.phazon_damage_modifier != PhazonDamageModifier::Default {
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("g_maxPhazonLagBeforeDamaging", version) + 4, {
                .float config.phazon_damage_per_sec;
            }),
        )?;
        let lin_off = if version == Version::Pal && version == Version::NtscJ {
            0x558
        } else {
            0x3ec
        };
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("UpdatePhazonDamage__7CPlayerFfR13CStateManager", version) + lin_off, {
            fmr f2, f0;
        }))?;
        if config.phazon_damage_modifier == PhazonDamageModifier::Linear {
            let del_off = if version == Version::Pal && version == Version::NtscJ {
                0x534
            } else {
                0x3c8
            };
            dol_patcher.ppcasm_patch(&ppcasm!(
                symbol_addr!("UpdatePhazonDamage__7CPlayerFfR13CStateManager", version) + del_off,
                {
                    nop;
                    nop;
                }
            ))?;
        }
    }

    Ok(())
}

// Make CMemory::Alloc return null on failure instead of calling rs_debugger_printf (which
// crashes via ErrorHandler/OSFatal, freezing the game before Alloc returns). The conditional
// branch at Alloc+0x64 (`bne .epilogue` if non-null, else fall into the printf) becomes an
// unconditional b to the epilogue, which still runs OSRestoreInterrupts and returns r31 (null).
// Callers handle the null via patch_build_async_null_guard and friends.
fn patch_alloc_null_on_failure(
    dol_patcher: &mut DolPatcher<'_>,
    version: Version,
) -> Result<(), String> {
    if matches!(
        version,
        Version::NtscK | Version::NtscUTrilogy | Version::NtscJTrilogy | Version::PalTrilogy
    ) {
        return Ok(());
    }

    let alloc_addr = symbol_addr!(
        "Alloc__7CMemoryFUlQ210IAllocator5EHintQ210IAllocator6EScopeQ210IAllocator5ETypeRC10CCallStack",
        version
    );

    dol_patcher.ppcasm_patch(&ppcasm!(alloc_addr + 0x64, {
        b {alloc_addr + 0x7C}; // bne .epilogue -> b .epilogue
    }))?;

    Ok(())
}

// Eliminate the ~1.5s freeze when CGameAllocator::Alloc can't find a block: the `beq skip_callback`
// at +0x284 becomes `b +0xA0` (the li r3,0 null return), skipping the OOM callback spin and
// DumpAllocations. Offsets identical across versions.
fn patch_alloc_oom_fast_fail(
    dol_patcher: &mut DolPatcher<'_>,
    version: Version,
) -> Result<(), String> {
    if matches!(
        version,
        Version::NtscK | Version::NtscUTrilogy | Version::NtscJTrilogy | Version::PalTrilogy
    ) {
        return Ok(());
    }

    let alloc_addr = symbol_addr!(
        "Alloc__14CGameAllocatorFUlQ210IAllocator5EHintQ210IAllocator6EScopeQ210IAllocator5ETypeRC10CCallStack",
        version
    );

    dol_patcher.ppcasm_patch(&ppcasm!(alloc_addr + 0x284, {
        b {alloc_addr + 0x284 + 0xA0}; // beq skip_callback -> b (li r3,0 null return)
    }))?;

    Ok(())
}

// Null-guard CResFactory::BuildAsync: on a null alloc, return early leaving *ppObj == null so
// IsLoaded() is false and the (already-guarded) Draw() skips it. Requires patch_alloc_null_on_failure.
fn patch_build_async_null_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    if matches!(
        version,
        Version::NtscK | Version::NtscUTrilogy | Version::NtscJTrilogy | Version::PalTrilogy
    ) {
        return Ok(());
    }

    let build_async_addr = symbol_addr!(
        "BuildAsync__11CResFactoryFRC10SObjectTagRC15CVParamTransferPP4IObj",
        version
    );
    let load_resource_async_addr =
        symbol_addr!("LoadResourceAsync__10CResLoaderFRC10SObjectTagPc", version);

    let cave_addr = emitter.emit_addressed(dol_patcher, |cave_addr| {
        ppcasm!(cave_addr, {
            cmpwi r29, 0x0;
            bne do_load;
            b {build_async_addr + 0xC8};                   // null: early return via epilogue
        do_load:
            mr r4, r26;                                    // displaced originals:
            addi r3, r25, 0x4;
            mr r5, r29;
            b {load_resource_async_addr};                  // tail call
        })
        .encoded_bytes()
    })?;

    // BuildAsync+0x6C: replace the 4 setup+call instrs with [bl cave | b +0x7C | nop | nop]; the
    // b +0x7C skips the dead slots to the mr r30,r3 that consumes the return value.
    dol_patcher.ppcasm_patch(&ppcasm!(build_async_addr + 0x6C, {
        bl {cave_addr};
        b {build_async_addr + 0x7C};
        nop;
        nop;
    }))?;

    Ok(())
}

// Retry instead of crash when the inflate output buffer (SLD_inner[0x20]) is null on OOM. On null the
// stub calls inflateEnd, clears the z_stream reference (56-byte struct leaked, one per OOM), sets the
// OOM flag, and returns 0 via the failure epilogue so AsyncIdle retries. Gated to PumpResource + inflateEnd.
fn patch_inflate_null_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    // The zlib decompress worker is unnamed; recover its start via the bl at PumpResource+0x5c.
    let (Some(pump), Some(inflate_end_addr)) = (
        symbol_addr_opt!("PumpResource__11CResFactoryFR12SLoadingData", version),
        symbol_addr_opt!("inflateEnd", version),
    ) else {
        return Ok(());
    };
    let worker: u32 = branch_target(dol_patcher.read_u32(pump + 0x5c)?, pump + 0x5c);

    let oom_flag_addr = { emitter.emit_addressed(dol_patcher, |_| 0u32.to_be_bytes().to_vec())? };

    let inflate_oom_site: u32 = worker + 0x200; // lwz r3, 0x20(r30) -- OOM intercept site
    let inflate_fail_addr: u32 = worker + 0x3c0; // worker failure epilogue: li r3,0; restore; blr

    // r30 = SLD_inner, r23 = z_stream ptr. Null buffer (OOM): tear down the stream and signal OOM so
    // the next retry re-allocates fresh; non-null: return r3 normally.
    let stub_addr = emitter.emit_addressed(dol_patcher, |cave_addr| {
        ppcasm!(cave_addr, {
            lwz r3, 0x20(r30);              // displaced: load inflate buf ptr
            cmpwi r3, 0x0;
            bne no_oom;
            mr r3, r23;
            bl {inflate_end_addr};
            li r0, 0;
            stb r0, 0x24(r30);             // release z_stream ownership
            stw r0, 0x28(r30);             // forget z_stream ptr
            lis r12, {oom_flag_addr}@h;    // signal OOM to Build's retry guard
            li r3, 0x1;
            stw r3, {oom_flag_addr}@l(r12);
            b {inflate_fail_addr};
        no_oom:
            blr;
        })
        .encoded_bytes()
    })?;

    dol_patcher.ppcasm_patch(&ppcasm!(inflate_oom_site, {
        bl { stub_addr };
    }))?;

    Ok(())
}

// Null-guard the medium-pool expansion in CGameAllocator::Alloc: the block passed to
// CMediumAllocPool::AddPuddle can be null (patch_alloc_null_on_failure), and AddPuddle would deref it
// and crash. On null, skip AddPuddle and fall through to the pool-retry path. Gated by symbol.
fn patch_add_puddle_null_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let Some(alloc) = symbol_addr_opt!(
        "Alloc__14CGameAllocatorFUlQ210IAllocator5EHintQ210IAllocator6EScopeQ210IAllocator5ETypeRC10CCallStack",
        version
    ) else {
        return Ok(());
    };

    // Expansion sequence at Alloc+0x1c4 (offset identical across versions): 5 setup instrs then
    // `bl AddPuddle`. AddPuddle's mangled name differs by version, so decode the branch target.
    let intercept_addr: u32 = alloc + 0x1c4;
    let add_puddle_addr: u32 = branch_target(
        dol_patcher.read_u32(intercept_addr + 0x14)?,
        intercept_addr + 0x14,
    );
    let retry_addr: u32 = intercept_addr + 0x18; // CMediumAllocPool::Alloc retry after AddPuddle

    // r3 = inner alloc result. Non-null: replay the 5 setup instrs and tail-call AddPuddle. Null:
    // blr, skipping AddPuddle. Both return to LR=intercept+4 (a b {retry_addr}).
    let stub_addr = emitter.emit_addressed(dol_patcher, |cave_addr| {
        ppcasm!(cave_addr, {
            cmpwi r3, 0x0;
            beq null_path;
            mr r0, r3;             // displaced originals:
            lwz r3, 0x74(r31);
            mr r5, r0;
            li r4, 0x1000;
            li r6, 0x1;
            b {add_puddle_addr};   // tail call
        null_path:
            blr;
        })
        .encoded_bytes()
    })?;

    // intercept_addr: 6 instrs (5 setup + bl AddPuddle) become [bl stub | b retry | nop x4].
    dol_patcher.ppcasm_patch(&ppcasm!(intercept_addr, {
        bl { stub_addr };
        b { retry_addr };
        nop;
        nop;
        nop;
        nop;
    }))?;

    Ok(())
}

// Null-guard CTexture::InitBitmapBuffers: on a null bitmap-buffer alloc, skip to the epilogue leaving
// x0c_bmpDataSize zeroed and the CARAMToken default (state=6); LoadToARAM returns 0 early for state==6.
// Requires patch_alloc_null_on_failure. Gated by symbol.
fn patch_init_bitmap_buffers_null_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let Some(func) = symbol_addr_opt!("InitBitmapBuffers__8CTextureF12ETexelFormatssi", version)
    else {
        return Ok(());
    };

    // Offsets identical across versions that name this. r3 = alloc result, r31 = this.
    let intercept_addr: u32 = func + 0x164; // first instr after bl Alloc
    let epilogue_addr: u32 = func + 0x180;
    emitter.emit_and_patch(dol_patcher, intercept_addr, true, |cave_addr| {
        ppcasm!(cave_addr, {
            cmpwi r3, 0x0;
            beq oom;
            lwz r5, 0xc(r31);  // displaced original; resume normal flow
            blr;
        oom:
            li r0, 0;
            stw r0, 0xc(r31); // zero x0c_bmpDataSize (no buffer allocated)
            b {epilogue_addr};
        })
        .encoded_bytes()
    })?;

    Ok(())
}

// Completes patch_init_bitmap_buffers_null_guard: with a null MRAM buffer the CTexture ctor's read
// loop would memcpy into address 0. On null, skip the read + MangleMipmap loops to InitTextureObjects
// (CPU-side setup, never touches the buffer); texture renders blank. Gated by symbol.
fn patch_texture_ctor_null_read_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let Some(func) = symbol_addr_opt!(
        "__ct__8CTextureFR12CInputStreamQ28CTexture11EAutoMipmapQ28CTexture9EBlackKey",
        version
    ) else {
        return Ok(());
    };

    // Offsets identical across versions that name this.
    let intercept_addr: u32 = func + 0x2c0; // mr r28, r3 -- buffer from GetMRAMSafe
    let init_tex_objects_addr: u32 = func + 0x344;
    emitter.emit_and_patch(dol_patcher, intercept_addr, true, |cave_addr| {
        ppcasm!(cave_addr, {
            mr    r28, r3;                   // displaced original: r28 = MRAM buffer
            cmpwi r28, 0x0;
            bne   ok;
            b     {init_tex_objects_addr};   // null: skip read + mangle loops
        ok:
            blr;
        })
        .encoded_bytes()
    })?;

    Ok(())
}

// Reduce peak heap during a beam switch by unloading the outgoing beam right after the swap instead of
// at kGS_OutWipeDone: the gun morph otherwise holds both beams at once, and an area transition in that
// window can drive free heap to zero. After kGS_InWipeDone only the new beam (x72c) is drawn, so the
// swap case's closing `b` becomes `bl stub` that unloads x730 and nulls it (OutWipeDone then skips, no
// double free). Gated by symbol.
fn patch_beam_switch_early_unload(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let Some(func) = symbol_addr_opt!("ProcessGunMorph__10CPlayerGunFfR13CStateManager", version)
    else {
        return Ok(());
    };

    // Offsets identical across versions that name this. r28 = this, r29 = mgr.
    let swap_break_addr: u32 = func + 0x138; // closing `b` of the kGS_InWipeDone swap
    let break_target: u32 = func + 0x19c;
    emitter.emit_and_patch(dol_patcher, swap_break_addr, true, |cave_addr| {
        ppcasm!(cave_addr, {
            lwz   r3, 0x730(r28);    // outgoingBeam
            cmplwi r3, 0x0;
            beq   done;              // nothing to free
            lwz   r0, 0x72c(r28);    // currentBeam
            cmplw r3, r0;
            beq   done;              // don't unload the live beam
            // outgoingBeam->Unload(mgr) via vtable[0x3c]
            lwz   r12, 0x0(r3);
            mr    r4, r29;
            lwz   r12, 0x3c(r12);
            mtctr r12;
            bctrl;
            li    r0, 0x0;
            stw   r0, 0x730(r28);    // outgoingBeam = NULL (OutWipeDone now skips)
        done:
            b     { break_target };
        })
        .encoded_bytes()
    })?;

    Ok(())
}

fn patch_morph_transition_oom_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let (Some(alloc), Some(can_enter_addr), Some(can_leave_addr)) = (
        symbol_addr_opt!("gGameAllocator", version),
        symbol_addr_opt!(
            "CanEnterMorphBallState__7CPlayerCFR13CStateManagerf",
            version
        ),
        symbol_addr_opt!(
            "CanLeaveMorphBallState__7CPlayerCFR13CStateManagerR9CVector3f",
            version
        ),
    ) else {
        return Ok(());
    };

    // Guard CanEnter/CanLeaveMorphBallState, not the Transition functions, to stay out of the crash
    // chain; on false the caller already plays the malfunction SFX and skips the partial-morph writes.
    // b-trampoline the first instr (LR unchanged), OOM path returns false; displaced stwu replayed.
    let free_bytes_addr: u32 = alloc + 0x90; // gGameAllocator x90_heapSize2 (free-bytes counter)
    let threshold: u32 = 704 * 1024;

    let can_enter_orig = dol_patcher.read_u32(can_enter_addr)?;
    emitter.emit_and_patch(dol_patcher, can_enter_addr, false, |cave_addr| {
        ppcasm!(cave_addr, {
            lis   r12, { free_bytes_addr }@h;
            lwz   r0,  { free_bytes_addr }@l(r12);
            lis   r12, { threshold }@h;
            cmplw r0, r12;
            bge   ok;
            li    r3, 0x0;                   // OOM: return false (caller plays malfunction SFX)
            blr;
        ok:
            .long can_enter_orig;            // trampoline: original first instruction
            b     { can_enter_addr + 4 };
        })
        .encoded_bytes()
    })?;

    let can_leave_orig = dol_patcher.read_u32(can_leave_addr)?;
    emitter.emit_and_patch(dol_patcher, can_leave_addr, false, |cave_addr| {
        ppcasm!(cave_addr, {
            lis   r12, { free_bytes_addr }@h;
            lwz   r0,  { free_bytes_addr }@l(r12);
            lis   r12, { threshold }@h;
            cmplw r0, r12;
            bge   ok;
            li    r3, 0x0;                   // OOM: return false (caller plays malfunction SFX)
            blr;
        ok:
            .long can_leave_orig;            // trampoline: original first instruction
            b     { can_leave_addr + 4 };
        })
        .encoded_bytes()
    })?;

    Ok(())
}

fn patch_draw_areas_oom_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let (Some(alloc), Some(func_addr)) = (
        symbol_addr_opt!("gGameAllocator", version),
        symbol_addr_opt!(
            "DrawAreas__9CMapWorldCFRCQ29CMapWorld18CMapWorldDrawParmsiRCQ24rstl64vector<Q29CMapWorld15CMapAreaBFSInfo,Q24rstl17rmemory_allocator>b",
            version
        ),
    ) else {
        return Ok(());
    };

    // DrawAreas renders the automap each frame via a local vector whose reserve(), under OOM, leaves
    // a null backing while size is bumped -> crash. Skip the whole function under low heap: it returns
    // void and is per-frame + stack-only, so it self-corrects when heap recovers.
    let free_bytes_addr: u32 = alloc + 0x90; // gGameAllocator x90_heapSize2 (free-bytes counter)
    let threshold: u32 = 256 * 1024;

    let func_orig = dol_patcher.read_u32(func_addr)?;
    emitter.emit_and_patch(dol_patcher, func_addr, false, |cave_addr| {
        ppcasm!(cave_addr, {
            lis   r12, { free_bytes_addr }@h;
            lwz   r0,  { free_bytes_addr }@l(r12);
            lis   r12, { threshold }@h;
            cmplw r0, r12;
            bge   ok;
            blr;                             // OOM: skip the whole automap draw this frame
        ok:
            .long func_orig;                 // trampoline: original first instruction
            b     { func_addr + 4 };
        })
        .encoded_bytes()
    })?;

    Ok(())
}

fn patch_change_weapon_oom_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let (Some(alloc), Some(change_weapon), Some(sfx_start_addr)) = (
        symbol_addr_opt!("gGameAllocator", version),
        symbol_addr_opt!(
            "ChangeWeapon__10CPlayerGunFRC12CPlayerStateR13CStateManager",
            version
        ),
        symbol_addr_opt!("SfxStart__11CSfxManagerFUsssbsbi", version),
    ) else {
        return Ok(());
    };

    // ChangeWeapon+0x78 is `beq +0xa4` (skip beam Load when loading==current). The stub honours the
    // original beq, then on OOM jumps to the epilogue (+0xf4), skipping the wipe so the morphing flag
    // is never set and the player can still fire. Offsets identical across versions.
    let intercept_addr: u32 = change_weapon + 0x78;
    let skip_target: u32 = change_weapon + 0xa4; // same-beam: original beq target (wipe)
    let epilogue_addr: u32 = change_weapon + 0xf4;
    let free_bytes_addr: u32 = alloc + 0x90; // gGameAllocator x90_heapSize2 (free-bytes counter)
    let threshold: u32 = 320 * 1024;

    let stub_addr = emitter.emit_addressed(dol_patcher, |cave_addr| {
        ppcasm!(cave_addr, {
            bne  no_orig_skip;                        // beams differ: check OOM
            b    { skip_target };                     // beams same: honour original beq
        no_orig_skip:
            lis  r12, { free_bytes_addr }@h;
            lwz  r0,  { free_bytes_addr }@l(r12);
            lis  r12, { threshold }@h;
            cmplw r0, r12;
            bge  no_oom;                              // enough memory: allow Load
            // OOM: HandleBeamChange set x2f8 |= 0x8 (morphing) which would freeze firing; restore
            // x2f8 = 1 (beam mode) since the morph that normally clears it never happens.
            li   r0, 0x1;
            stw  r0, 0x2f8(r29);
            // malfunction SFX; mini-frame holds the CSfxHandle out-param, popped before the jump
            stwu r1, -0x20(r1);
            addi r3, r1, 0x10;                       // CSfxHandle out buffer
            li   r4, 0x6f5;                          // SFXsam_b_malfxn_00
            li   r5, 0x7f;                           // vol
            li   r6, 0x40;                           // pan
            li   r7, 0x1;                            // useAcoustics
            li   r8, 0x40;                           // priority
            li   r9, 0x0;                            // not looped
            li   r10, -1;                            // areaId (all)
            bl   { sfx_start_addr };
            addi r1, r1, 0x20;
            b    { epilogue_addr };                   // cancel beam switch
        no_oom:
            blr;                                      // fall through to Load block
        })
        .encoded_bytes()
    })?;

    dol_patcher.ppcasm_patch(&ppcasm!(intercept_addr, {
        bl { stub_addr };
    }))?;

    Ok(())
}

fn patch_logbook_oom_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let Some(alloc) = symbol_addr_opt!("gGameAllocator", version) else {
        return Ok(());
    };

    // The Log Book ctor force-builds every world pak directory and a scan token per scan with no OOM
    // retry -> null deref on alloc failure. Gate on low TOTAL free (threshold 0x10000-aligned).
    let free_bytes_addr: u32 = alloc + 0x90;
    let total_threshold: u32 = 2304 * 1024;

    // 0-00/0-01: the frame is bound before the two heavy ctor calls, so skip the pair on low free for
    // a degraded (framed, no assets) page rather than a blank one.
    if matches!(version, Version::NtscU0_00 | Version::NtscU0_01) {
        let (Some(ctor), Some(ensure)) = (
            symbol_addr_opt!(
                "__ct__14CLogBookScreenFRC13CStateManagerRC9CGuiFrameRC12CStringTable",
                version
            ),
            symbol_addr_opt!("EnsureWorldPaksReady__5CMainFv", version),
        ) else {
            return Ok(());
        };
        let intercept_addr: u32 = ctor + 0x118; // bl EnsureWorldPaksReady; then bl InitializeLogBook
        let resume_addr: u32 = ctor + 0x11c;
        let epilogue_addr: u32 = ctor + 0x124;
        emitter.emit_and_patch(dol_patcher, intercept_addr, false, |cave_addr| {
            ppcasm!(cave_addr, {
                lis   r12, { free_bytes_addr }@h;
                lwz   r0,  { free_bytes_addr }@l(r12);
                lis   r12, { total_threshold }@h;
                cmplw r0, r12;
                blt   frame_only;
                bl    { ensure };                 // displaced call
                b     { resume_addr };
            frame_only:
                b     { epilogue_addr };
            })
            .encoded_bytes()
        })?;
        return Ok(());
    }

    // Other versions lack the ctor symbols; deny the whole subscreen (StartTransition tolerates null).
    let Some(func) = symbol_addr_opt!(
        "BuildPauseSubScreen__12CPauseScreenFQ212CPauseScreen10ESubScreenRC13CStateManagerRC9CGuiFrame",
        version
    ) else {
        return Ok(());
    };
    let intercept_addr: u32 = func + 0x4c;
    let return_null_addr: u32 = func + 0x108;

    let intercept_orig = dol_patcher.read_u32(intercept_addr)?;
    emitter.emit_and_patch(dol_patcher, intercept_addr, false, |cave_addr| {
        ppcasm!(cave_addr, {
            lis   r12, { free_bytes_addr }@h;
            lwz   r0,  { free_bytes_addr }@l(r12);
            lis   r12, { total_threshold }@h;
            cmplw r0, r12;
            blt   oom;
            .long intercept_orig;             // displaced first instruction
            b     { intercept_addr + 4 };
        oom:
            b     { return_null_addr };
        })
        .encoded_bytes()
    })?;

    Ok(())
}

// Defer a ready resource build when free heap is below threshold, so the heavy build waits for memory
// and caches cleanly instead of finishing degraded and sticking (a beam built in a transient low-heap
// window stays broken and wedges the morph). PumpResource already defers not-ready resources; this
// extends that to ready ones. Never defer the synchronous Build path (r31 == 0), which spins until
// nonzero. Gated to PumpResource.
fn patch_pump_resource_oom_defer(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let (Some(pump), Some(alloc)) = (
        symbol_addr_opt!("PumpResource__11CResFactoryFR12SLoadingData", version),
        symbol_addr_opt!("gGameAllocator", version),
    ) else {
        return Ok(());
    };

    // Offsets verified identical across all versions that have this symbol.
    let intercept_addr: u32 = pump + 0x40; // beq <defer> (the readiness-defer branch)
    let not_ready_defer: u32 = pump + 0x114; // li r3,0; epilogue (return 0 -> node stays queued)
    let proceed_addr: u32 = pump + 0x44; // original fall-through: build the resource
    let free_bytes_addr: u32 = alloc + 0x90; // gGameAllocator x90_heapSize2 (free-bytes counter)
    let threshold: u32 = 576 * 1024;

    emitter.emit_and_patch(dol_patcher, intercept_addr, true, |cave_addr| {
        ppcasm!(cave_addr, {
            beq   defer;                 // original: resource not ready -> defer
            cmpwi r31, 0x0;              // r31 = time budget (0 = synchronous Build)
            beq   proceed;               // sync path: never memory-defer (would spin forever)
            lis   r12, { free_bytes_addr }@h;
            lwz   r0,  { free_bytes_addr }@l(r12);
            lis   r12, { threshold }@h;
            cmplw r0, r12;
            blt   defer;                 // low memory: defer
        proceed:
            b     { proceed_addr };
        defer:
            b     { not_ready_defer };   // return 0 -> node stays queued, retried next pump
        })
        .encoded_bytes()
    })?;

    Ok(())
}

// Recover the decompressor from a failed output-buffer alloc instead of wedging: under fragmentation
// this contiguous alloc can return null even when the total-free defer guard passes, and the worker
// would then resume the already-set-up stream into a null buffer forever (the beam-morph wedge). On
// null, tear the stream down like the worker's own cleanup (inflateEnd, then Free when owned), clear
// it, and return 0 via the failure epilogue so the next pump re-allocates and recovers.
fn patch_inflate_buffer_oom_recover(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    // Same unnamed worker as patch_inflate_null_guard: recover its start via the bl at PumpResource+0x5c.
    let (Some(pump), Some(inflate_end), Some(free_addr)) = (
        symbol_addr_opt!("PumpResource__11CResFactoryFR12SLoadingData", version),
        symbol_addr_opt!("inflateEnd", version),
        symbol_addr_opt!("Free__7CMemoryFPCv", version),
    ) else {
        return Ok(());
    };
    let worker: u32 = branch_target(dol_patcher.read_u32(pump + 0x5c)?, pump + 0x5c);

    let intercept_addr: u32 = worker + 0x138; // neg r0, r3  (right after the output-buffer Alloc)
    let continue_addr: u32 = worker + 0x13c; // original next instruction
    let return_zero: u32 = worker + 0x3c0; // li r3,0; epilogue (return "not done")

    emitter.emit_and_patch(dol_patcher, intercept_addr, false, |cave_addr| {
        ppcasm!(cave_addr, {
            cmpwi r3, 0x0;                   // buffer alloc result
            bne   ok;                        // non-null -> proceed normally
            // null buffer: tear down the stream so the next pump retries from scratch
            mr    r3, r23;                   // zlib stream
            bl    { inflate_end };
            lbz   r0, 0x24(r30);             // stream "owned" flag
            cmpwi r0, 0x0;
            beq   skip_free;
            lwz   r3, 0x28(r30);             // stream ptr
            bl    { free_addr };
        skip_free:
            li    r0, 0x0;
            stw   r0, 0x28(r30);             // clear stream optional (ptr)
            stb   r0, 0x24(r30);             //   and owned flag -> next call re-does setup
            b     { return_zero };           // return 0 (not done); node stays pending
        ok:
            neg   r0, r3;                    // displaced original instruction
            b     { continue_addr };
        })
        .encoded_bytes()
    })?;

    Ok(())
}

fn patch_meta(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    config: &PatchConfig,
) -> Result<(), String> {
    patch_rel_loader(dol_patcher, emitter, version)?;

    {
        let build_info_address: u32 = match version {
            Version::NtscU0_00 => 0x803cc588,
            Version::NtscU0_01 => 0x803cc768,
            Version::NtscU0_02 => 0x803cd648,
            Version::NtscK => 0x803cc688,
            Version::NtscJ => 0x803b86cc,
            Version::Pal => 0x803b6924,
            _ => panic!("This version of the game does not support etching a UUID into the dol"),
        };
        let build_info_address = build_info_address + "!#$Met".len() as u32;
        dol_patcher.patch(build_info_address, config.uuid.to_vec().into())?;
    }

    if version == Version::Pal || version == Version::NtscJ {
        dol_patcher.patch(
            symbol_addr!("aMetroidprime", version),
            b"randomprime\0"[..].into(),
        )?;
    } else {
        dol_patcher
            .patch(
                symbol_addr!("aMetroidprimeA", version),
                b"randomprime A\0"[..].into(),
            )?
            .patch(
                symbol_addr!("aMetroidprimeB", version),
                b"randomprime B\0"[..].into(),
            )?;
    }

    if config.automatic_crash_screen {
        let off = if version == Version::NtscU0_00 {
            0xEC
        } else {
            0x120
        };
        dol_patcher.ppcasm_patch(&ppcasm!(
            symbol_addr!("CrashScreenControllerPollBranch", version) + off,
            {
                nop;
            }
        ))?;
    }

    if config.skip_splash_screens {
        dol_patcher.ppcasm_patch(&ppcasm!(
            symbol_addr!(
                "__ct__13CSplashScreenFQ213CSplashScreen13ESplashScreen",
                version
            ) + 0x70,
            {
                nop;
            }
        ))?;
    }

    if config.multiworld_dol_patches {
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("IncrPickUpSwitchCaseData", version) + 21 * 4, {
            .long symbol_addr!("IncrPickUp__12CPlayerStateFQ212CPlayerState9EItemTypei", version) + 25 * 4;
        }))?;
        dol_patcher.ppcasm_patch(&ppcasm!(
            symbol_addr!(
                "DecrPickUp__12CPlayerStateFQ212CPlayerState9EItemTypei",
                version
            ) + 5 * 4,
            {
                nop;
                nop;
                nop;
                nop;
                nop;
                nop;
                nop;
            }
        ))?;
    }

    // The hint-state array is repurposed as save-slot scratch (see the save UUID feature), so neuter
    // UpdateHintState to a blr so the game never reads it. A caller-supplied replacement takes
    // precedence (Randovania's remote-execution hook does not touch the hint-state vector at
    // CGameState+0x1f8). Only suppresses in-game HUD-memo hint popups; artifact totem scans are a
    // separate logbook path and are unaffected.
    if let Some(bytes) = &config.update_hint_state_replacement {
        dol_patcher.patch(
            symbol_addr!("UpdateHintState__13CStateManagerFf", version),
            Cow::from(bytes.clone()),
        )?;
    } else if let Some(addr) = symbol_addr_opt!("UpdateHintState__13CStateManagerFf", version) {
        dol_patcher.ppcasm_patch(&ppcasm!(addr, {
            blr;
        }))?;
    }

    Ok(())
}

fn patch_bss_heap_extension(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    // Inject the unused ~80KB BSS gap in CMetroidAreaCollider's sDupVertexList array (tail never
    // populated, ends at sDupEdgeList) into the heap free pool: hook CGameAllocator::Initialize's
    // epilogue blr to build a standalone free-block chain (head -> tail sentinel) and register it
    // via AddFreeEntryToFreeList. The gap is derived from the boundary symbols, and the array shape
    // (0x19000 size, 0x5000 used prefix, no intervening symbols) is identical across versions.
    let (Some(vertex_list), Some(edge_list), Some(game_allocator_addr), Some(add_free_entry_addr)) = (
        symbol_addr_opt!("sDupVertexList__20CMetroidAreaCollider", version),
        symbol_addr_opt!("sDupEdgeList__20CMetroidAreaCollider", version),
        symbol_addr_opt!("gGameAllocator", version),
        symbol_addr_opt!(
            "AddFreeEntryToFreeList__14CGameAllocatorFPQ214CGameAllocator12SGameMemInfo",
            version
        ),
    ) else {
        return Ok(());
    };
    let Some(init) = symbol_addr_opt!("Initialize__14CGameAllocatorFR10COsContext", version) else {
        return Ok(());
    };

    // Both blocks must be 32-byte aligned (SGameMemInfo pointer fields use & ~31 masking). Raw gap
    // [vertex_list + 0x5000, edge_list) snapped inward to 32 bytes.
    let head_addr: u32 = (vertex_list + 0x5000 + 0x1f) & !0x1f;
    let tail_addr: u32 = (edge_list - 0x20) & !0x1f;
    let free_len: u32 = tail_addr - head_addr - 0x20; // usable bytes in the registered free block
    let heap_counter_addr: u32 = game_allocator_addr + 0x90; // gGameAllocator + 0x90 = x90_heapSize2
    let init_blr_addr = init + 0x384;

    let cave_addr = emitter.emit_addressed(dol_patcher, |cave_addr| {
        ppcasm!(cave_addr, {
            // Initialize already tore down its own frame before the replaced blr; mini-frame for our bl.
            stwu r1, -0x10(r1);
            mflr r0;
            stw  r0, 0x14(r1);

            // Build head SGameMemInfo at head_addr
            lis  r12, { head_addr }@h;
            addi r12, r12, { head_addr }@l;
            lis  r0, 0xEFEF;
            ori  r0, r0, 0xEFEF;
            stw  r0, 0x00(r12);        // x0_priorGuard = 0xEFEFEFEF
            lis  r0, { (free_len >> 16) as i32 };
            ori  r0, r0, { (free_len & 0xFFFF) as i32 };
            stw  r0, 0x04(r12);        // x4_len = free_len (tail_addr - head_addr - 0x20)
            li   r0, 0;
            stw  r0, 0x08(r12);        // x8_fileAndLine = 0
            stw  r0, 0x0C(r12);        // xc_type = 0
            stw  r0, 0x10(r12);        // x10_prev = 0 (not allocated, no prior block)
            // Use lis+ori, not addi: r0 as addi destination is the PPC zero special case.
            lis  r0, { (tail_addr >> 16) as i32 };
            ori  r0, r0, { (tail_addr & 0xFFFF) as i32 };
            stw  r0, 0x14(r12);        // x14_next = tail_addr
            li   r0, 0;
            stw  r0, 0x18(r12);        // x18_nextFree = 0 (set by AddFreeEntryToFreeList)
            lis  r0, 0xEAEA;
            ori  r0, r0, 0xEAEA;
            stw  r0, 0x1C(r12);        // x1c_postGuard = 0xEAEAEAEA

            // Build tail sentinel SGameMemInfo at tail_addr (len=0, next=null)
            lis  r12, { tail_addr }@h;
            addi r12, r12, { tail_addr }@l;
            lis  r0, 0xEFEF;
            ori  r0, r0, 0xEFEF;
            stw  r0, 0x00(r12);        // x0_priorGuard = 0xEFEFEFEF
            li   r0, 0;
            stw  r0, 0x04(r12);        // x4_len = 0 (sentinel: never selected by FindFreeBlock)
            stw  r0, 0x08(r12);        // x8_fileAndLine = 0
            stw  r0, 0x0C(r12);        // xc_type = 0
            lis  r0, { (head_addr >> 16) as i32 };
            ori  r0, r0, { (head_addr & 0xFFFF) as i32 };
            stw  r0, 0x10(r12);        // x10_prev = head_addr (bit0=0, not allocated)
            li   r0, 0;
            stw  r0, 0x14(r12);        // x14_next = 0 (null; guards forward-coalesce path)
            stw  r0, 0x18(r12);        // x18_nextFree = 0
            lis  r0, 0xEAEA;
            ori  r0, r0, 0xEAEA;
            stw  r0, 0x1C(r12);        // x1c_postGuard = 0xEAEAEAEA

            // x90_heapSize2 += free_len (keep OOM debug counter accurate)
            lis  r12, { heap_counter_addr }@h;
            lwz  r0, { heap_counter_addr }@l(r12);
            lis  r6, { (free_len >> 16) as i32 };
            ori  r6, r6, { (free_len & 0xFFFF) as i32 }; // r6 = free_len
            add  r0, r0, r6;
            stw  r0, { heap_counter_addr }@l(r12);

            // AddFreeEntryToFreeList(this=gGameAllocator, info=head)
            lis  r3, { game_allocator_addr }@h;
            addi r3, r3, { game_allocator_addr }@l;
            lis  r4, { head_addr }@h;
            addi r4, r4, { head_addr }@l;
            bl   { add_free_entry_addr };

            lwz  r0, 0x14(r1);
            mtlr r0;
            addi r1, r1, 0x10;
            blr;
        })
        .encoded_bytes()
    })?;

    dol_patcher.ppcasm_patch(&ppcasm!(init_blr_addr, {
        b { cave_addr };
    }))?;

    Ok(())
}

fn patch_heap_optimization(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    #[allow(unused_variables)] config: &PatchConfig,
) -> Result<(), String> {
    if !config.qol_game_breaking {
        return Ok(());
    }

    /* Patch heap allocator to tolerate failed allocations (return nullptr instead of panic) */
    patch_alloc_null_on_failure(dol_patcher, version)?;
    patch_alloc_oom_fast_fail(dol_patcher, version)?;
    patch_add_puddle_null_guard(dol_patcher, emitter, version)?;

    /* Patch to deny memory-hungry actions if heap below danger threshold */
    patch_morph_transition_oom_guard(dol_patcher, emitter, version)?;
    patch_draw_areas_oom_guard(dol_patcher, emitter, version)?;
    patch_logbook_oom_guard(dol_patcher, emitter, version)?;
    patch_change_weapon_oom_guard(dol_patcher, emitter, version)?;

    /* Reduce peak heap during beam switch by freeing the outgoing beam at the morph swap */
    patch_beam_switch_early_unload(dol_patcher, emitter, version)?;

    /* Inject unused 80KB BSS gap into heap free pool (NTSC 0-00 only) */
    patch_bss_heap_extension(dol_patcher, emitter, version)?;

    /* Patch alloc call sites to tolerate nullptr return values */
    patch_build_async_null_guard(dol_patcher, emitter, version)?; // Solves start menu crash
    patch_inflate_null_guard(dol_patcher, emitter, version)?;
    patch_init_bitmap_buffers_null_guard(dol_patcher, emitter, version)?;
    patch_texture_ctor_null_read_guard(dol_patcher, emitter, version)?;
    patch_pump_resource_oom_defer(dol_patcher, emitter, version)?;

    /* Recover the decompressor from a fragmentation-failed output-buffer alloc instead of wedging */
    patch_inflate_buffer_oom_recover(dol_patcher, emitter, version)?;

    Ok(())
}

fn should_patch_smoother_reposition(config: &PatchConfig) -> bool {
    let mut smoother_reposition = false;
    for (_, level) in config.level_data.iter() {
        if smoother_reposition {
            break;
        }
        for (_, room) in level.rooms.iter() {
            if smoother_reposition {
                break;
            }
            if room.doors.is_none() {
                continue;
            }
            for (_, door) in room.doors.as_ref().unwrap().iter() {
                if door.destination.is_some() {
                    smoother_reposition = true;
                    break;
                }
            }
        }
    }

    smoother_reposition
}

fn patch_gameplay_tweaks(
    dol_patcher: &mut DolPatcher<'_>,
    _emitter: &mut TextEmitter,
    version: Version,
    config: &PatchConfig,
) -> Result<(), String> {
    if config.shoot_in_grapple {
        let off = if [Version::NtscJ, Version::Pal].contains(&version) {
            0x324
        } else {
            0x330
        };
        dol_patcher.ppcasm_patch(&ppcasm!(
            symbol_addr!(
                "UpdateGrappleState__7CPlayerFRC11CFinalInputR13CStateManager",
                version
            ) + off,
            {
                nop;
            }
        ))?;
    }

    if config.qol_general {
        dol_patcher.ppcasm_patch(&ppcasm!(
            symbol_addr!("FireSecondary__10CPlayerGunFfR13CStateManager", version) + 0x78,
            {
                lwz         r0, 0x2f8(r30);
                rlwinm.     r0, r0, 0, 29, 29;
                beq         { symbol_addr!("FireSecondary__10CPlayerGunFfR13CStateManager", version) + 0xA8 };
                lwz         r0, 0x310(r30);
                cmpwi       r0, 2;
                bne         { symbol_addr!("FireSecondary__10CPlayerGunFfR13CStateManager", version) + 0xC8 };
                lbz         r0, 0x832(r30);
                rlwinm.     r0, r0, 27, 31, 31;
                beq         { symbol_addr!("FireSecondary__10CPlayerGunFfR13CStateManager", version) + 0xC8 };
                lbz         r0, 0x833(r30);
                rlwinm.     r0, r0, 30, 31, 31;
                beq         { symbol_addr!("FireSecondary__10CPlayerGunFfR13CStateManager", version) + 0xC8 };
            }
        ))?;
    }

    if should_patch_smoother_reposition(config) {
        dol_patcher.ppcasm_patch(&ppcasm!(
            symbol_addr!(
                "Teleport__7CPlayerFRC12CTransform4fR13CStateManagerb",
                version
            ) + 0x31C,
            {
                nop;
            }
        ))?;
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("SetSpawnedMorphBallState__7CPlayerFQ27CPlayer21EPlayerMorphBallStateR13CStateManager", version) + 0x24, { nop; }))?;
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("SetSpawnedMorphBallState__7CPlayerFQ27CPlayer21EPlayerMorphBallStateR13CStateManager", version) + 0x104, { nop; }))?;
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("SetSpawnedMorphBallState__7CPlayerFQ27CPlayer21EPlayerMorphBallStateR13CStateManager", version) + 0xf8, { nop; }))?;
    }

    if config.escape_sequence_counts_up {
        let patch = ppcasm!(symbol_addr!("UpdateEscapeSequenceTimer__13CStateManagerFf", version) + 0x30, {
            fadds f2, f2, f1;
        });
        dol_patcher.ppcasm_patch(&patch)?;
        let patch = ppcasm!(symbol_addr!("UpdateEscapeSequenceTimer__13CStateManagerFf", version) + 0xb4, {
            b { symbol_addr!("UpdateEscapeSequenceTimer__13CStateManagerFf", version) + 0x164 };
        });
        dol_patcher.ppcasm_patch(&patch)?;
        let patch_offset = if version == Version::Pal || version == Version::NtscJ {
            0xb84
        } else {
            0xaf8
        };
        let patch = ppcasm!(
            symbol_addr!("Update__9CSamusHudFfRC13CStateManagerUibb", version) + patch_offset,
            { nop }
        );
        dol_patcher.ppcasm_patch(&patch)?;
    }

    if config.nonvaria_heat_damage {
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("ThinkAreaDamage__22CScriptSpecialFunctionFfR13CStateManager", version) + 0x4c, {
            lwz r4, 0xdc(r4); nop; subf r0, r6, r5; cntlzw r0, r0; nop;
        }))?;
    }

    match config.staggered_suit_damage {
        SuitDamageReduction::Progressive => {
            let (po, jo) = if version == Version::Pal || version == Version::NtscJ {
                (0x11c, 0x1b8)
            } else {
                (0x128, 0x1c4)
            };
            dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("ApplyLocalDamage__13CStateManagerFRC9CVector3fRC9CVector3fR6CActorfRC11CWeaponMode", version) + po, {
                lwz r3, 0x8b8(r25); lwz r3, 0(r3); lwz r4, 220(r3);
                lwz r5, 212(r3); addc r4, r4, r5; lwz r5, 228(r3); addc r4, r4, r5;
                rlwinm r4, r4, 2, 0, 29; lis r6, data@h; addi r6, r6, data@l;
                lfsx f0, r4, r6;
                b { symbol_addr!("ApplyLocalDamage__13CStateManagerFRC9CVector3fRC9CVector3fR6CActorfRC11CWeaponMode", version) + jo };
                data: .float 0.0; .float 0.1; .float 0.2; .float 0.5;
            }))?;
        }
        SuitDamageReduction::Additive => {
            let (po, jo) = if version == Version::Pal || version == Version::NtscJ {
                (0x11c, 0x1b8)
            } else {
                (0x128, 0x1c4)
            };
            dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("ApplyLocalDamage__13CStateManagerFRC9CVector3fRC9CVector3fR6CActorfRC11CWeaponMode", version) + po, {
                lwz r3, 0x8b8(r25); lwz r3, 0(r3); lwz r4, 220(r3);
                lwz r5, 212(r3); slwi r5, r5, 1; or r4, r4, r5;
                lwz r5, 228(r3); slwi r5, r5, 2; or r4, r4, r5;
                rlwinm r4, r4, 2, 0, 29; lis r6, data@h; addi r6, r6, data@l;
                lfsx f0, r4, r6;
                b { symbol_addr!("ApplyLocalDamage__13CStateManagerFRC9CVector3fRC9CVector3fR6CActorfRC11CWeaponMode", version) + jo };
                data: .float 0.0; .float 0.1; .float 0.1; .float 0.2; .float 0.3; .float 0.4; .float 0.4; .float 0.5;
            }))?;
        }
        SuitDamageReduction::Default => {}
    }

    for (pickup_type, value) in &config.item_max_capacity {
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("CPlayerState_PowerUpMaxValues", version) + pickup_type.kind() * 4, {
            .long *value;
        }))?;
    }

    for (missile_type, cost) in &config.missile_costs {
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("CPlayerState_MissileCostValues", version) + missile_type * 4, {
                .long *cost;
            }),
        )?;
    }

    let etank_capacity = config.etank_capacity as f32;
    dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("g_EtankCapacity", version), {
        .float etank_capacity;
        .float { etank_capacity - 1.0 };
    }))?;

    Ok(())
}

fn patch_cosmetic(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    config: &PatchConfig,
) -> Result<(), String> {
    let remove_ball_color = config.ctwk_config.morph_ball_size.unwrap_or(1.0) < 0.999;

    if remove_ball_color {
        let colors = b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00".to_vec();
        dol_patcher.patch(
            symbol_addr!("skBallInnerGlowColors", version),
            colors.clone().into(),
        )?;
        dol_patcher.patch(
            symbol_addr!("BallAuxGlowColors", version),
            colors.clone().into(),
        )?;
        dol_patcher.patch(
            symbol_addr!("BallTransFlashColors", version),
            colors.clone().into(),
        )?;
        dol_patcher.patch(
            symbol_addr!("BallSwooshColors", version),
            colors.clone().into(),
        )?;
        dol_patcher.patch(
            symbol_addr!("BallSwooshColorsJaggy", version),
            colors.clone().into(),
        )?;
        dol_patcher.patch(
            symbol_addr!("BallSwooshColorsCharged", version),
            colors.clone().into(),
        )?;
        dol_patcher.patch(symbol_addr!("BallGlowColors", version), colors.into())?;
    } else if let Some(suit_colors) = config.suit_colors.as_ref() {
        let mut colors: Vec<Vec<u8>> = vec![
            vec![
                0xc2, 0x7e, 0x10, 0x66, 0xc4, 0xff, 0x60, 0xff, 0x90, 0x33, 0x33, 0xff, 0xff, 0x80,
                0x80, 0x00, 0x9d, 0xb6, 0xd3, 0xf1, 0x00, 0x60, 0x33, 0xff, 0xfb, 0x98, 0x21,
            ],
            vec![
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xd5,
                0x19, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            ],
            vec![
                0xc2, 0x7e, 0x10, 0x66, 0xc4, 0xff, 0x60, 0xff, 0x90, 0x33, 0x33, 0xff, 0xff, 0x20,
                0x20, 0x00, 0x9d, 0xb6, 0xd3, 0xf1, 0x00, 0xa6, 0x86, 0xd8, 0xfb, 0x98, 0x21,
            ],
            vec![
                0xC2, 0x8F, 0x17, 0x70, 0xD4, 0xFF, 0x6A, 0xFF, 0x8A, 0x3D, 0x4D, 0xFF, 0xC0, 0x00,
                0x00, 0x00, 0xBE, 0xDC, 0xDF, 0xFF, 0x00, 0xC4, 0x9E, 0xFF, 0xFF, 0x9A, 0x22,
            ],
            vec![
                0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00, 0xFF, 0xD5,
                0x19, 0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00,
            ],
            vec![
                0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00, 0xFF, 0x80,
                0x20, 0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00,
            ],
            vec![
                0xc2, 0x7e, 0x10, 0x66, 0xc4, 0xff, 0x6c, 0xff, 0x61, 0x33, 0x33, 0xff, 0xff, 0x20,
                0x20, 0x00, 0x9d, 0xb6, 0xd3, 0xf1, 0x00, 0xa6, 0x86, 0xd8, 0xfb, 0x98, 0x21,
            ],
        ];
        for color in colors.iter_mut() {
            for j in 0..9 {
                let angle = if [0].contains(&j) && suit_colors.power_deg.is_some() {
                    suit_colors.power_deg.unwrap()
                } else if [1, 2].contains(&j) && suit_colors.varia_deg.is_some() {
                    suit_colors.varia_deg.unwrap()
                } else if [3].contains(&j) && suit_colors.gravity_deg.is_some() {
                    suit_colors.gravity_deg.unwrap()
                } else if [4].contains(&j) && suit_colors.phazon_deg.is_some() {
                    suit_colors.phazon_deg.unwrap()
                } else {
                    0
                };
                let angle = angle % 360;
                if angle == 0 {
                    continue;
                }
                let matrix = huerotate_matrix(angle as f32);
                let r_idx = j * 3;
                let new_rgb =
                    huerotate_color(matrix, color[r_idx], color[r_idx + 1], color[r_idx + 2]);
                color[r_idx] = new_rgb[0];
                color[r_idx + 1] = new_rgb[1];
                color[r_idx + 2] = new_rgb[2];
            }
        }
        let addrs = [
            symbol_addr!("skBallInnerGlowColors", version),
            symbol_addr!("BallAuxGlowColors", version),
            symbol_addr!("BallTransFlashColors", version),
            symbol_addr!("BallSwooshColors", version),
            symbol_addr!("BallSwooshColorsJaggy", version),
            symbol_addr!("BallSwooshColorsCharged", version),
            symbol_addr!("BallGlowColors", version),
        ];
        for (addr, color) in addrs.iter().zip(colors.into_iter()) {
            dol_patcher.patch(*addr, color.into())?;
        }
    }

    if config.qol_cosmetic {
        if version != Version::Pal && version != Version::NtscJ {
            dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("SetNumMissiles__20CHudMissileInterfaceFiRC13CStateManager", version) + 0x14, {
                b skip; fmt: .asciiz b"%03d/%03d"; skip:
                stw r30, 40(r1); mr r30, r3; stw r4, 8(r1); lwz r6, 4(r30);
                mr r5, r4; lis r4, fmt@h; addi r4, r4, fmt@l; addi r3, r1, 12;
                nop; bl { symbol_addr!("sprintf", version) }; addi r3, r1, 20; addi r4, r1, 12;
            }))?;
        }
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("SetBombParams__17CHudBallInterfaceFiiibbb", version) + 0x2c, {
                b skip; fmt: .asciiz b"%d/%d"; nop; skip:
                mr r6, r27; mr r5, r28; lis r4, fmt@h; addi r4, r4, fmt@l;
                addi r3, r1, 12; nop; bl { symbol_addr!("sprintf", version) };
            }),
        )?;
    }

    let is_memory_relay_active_func =
        patch_emit_is_memory_relay_active_func(dol_patcher, emitter, version)?;
    patch_set_pickup_icon_txtr(dol_patcher, emitter, version, is_memory_relay_active_func)?;

    Ok(())
}

fn patch_game_options(
    dol_patcher: &mut DolPatcher<'_>,
    version: Version,
    config: &PatchConfig,
) -> Result<(), String> {
    {
        let mut screen_brightness: u32 = 4;
        let mut screen_offset_x: i32 = 0;
        let mut screen_offset_y: i32 = 0;
        let mut screen_stretch: i32 = 0;
        let mut sound_mode: u32 = 1;
        let mut sfx_volume: u32 = 0x7f;
        let mut music_volume: u32 = 0x7f;
        let mut visor_opacity: u32 = 0xff;
        let mut helmet_opacity: u32 = 0xff;
        let mut hud_lag: bool = true;
        let mut reverse_y_axis: bool = false;
        let mut rumble: bool = true;
        let mut swap_beam_controls: bool = false;
        let hints: bool = false;
        if let Some(opts) = config.default_game_options.clone() {
            if let Some(v) = opts.screen_brightness {
                screen_brightness = v;
            }
            if let Some(v) = opts.screen_offset_x {
                screen_offset_x = v;
            }
            if let Some(v) = opts.screen_offset_y {
                screen_offset_y = v;
            }
            if let Some(v) = opts.screen_stretch {
                screen_stretch = v;
            }
            if let Some(v) = opts.sound_mode {
                sound_mode = v;
            }
            if let Some(v) = opts.sfx_volume {
                sfx_volume = v;
            }
            if let Some(v) = opts.music_volume {
                music_volume = v;
            }
            if let Some(v) = opts.visor_opacity {
                visor_opacity = v;
            }
            if let Some(v) = opts.helmet_opacity {
                helmet_opacity = v;
            }
            if let Some(v) = opts.hud_lag {
                hud_lag = v;
            }
            if let Some(v) = opts.reverse_y_axis {
                reverse_y_axis = v;
            }
            if let Some(v) = opts.rumble {
                rumble = v;
            }
            if let Some(v) = opts.swap_beam_controls {
                swap_beam_controls = v;
            }
        }
        let mut bit_flags: u32 = 0x00;
        if hud_lag {
            bit_flags |= 1 << 7;
        }
        if reverse_y_axis {
            bit_flags |= 1 << 6;
        }
        if rumble {
            bit_flags |= 1 << 5;
        }
        if swap_beam_controls {
            bit_flags |= 1 << 4;
        }
        if hints {
            bit_flags |= 1 << 3;
        }
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("ResetToDefaults__12CGameOptionsFv", version) + 9 * 4, {
                li r0, screen_brightness; stw r0, 0x48(r3);
                li r0, screen_offset_x;  stw r0, 0x4C(r3);
                li r0, screen_offset_y;  stw r0, 0x50(r3);
                li r0, screen_stretch;   stw r0, 0x54(r3);
                li r0, sfx_volume;       stw r0, 0x58(r3);
                li r0, music_volume;     stw r0, 0x5C(r3);
                li r0, sound_mode;       stw r0, 0x44(r3);
                li r0, visor_opacity;    stw r0, 0x60(r3);
                li r0, helmet_opacity;   stw r0, 0x64(r3);
                li r0, bit_flags;        stb r0, 0x68(r3);
                nop; nop; nop; nop; nop;
            }),
        )?;
    }

    if version == Version::Pal {
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("__ct__14CSystemOptionsFv", version) + 0x1dc, {
                li r6, 100; stw r6, 0x80(r31); lis r6, 0xF7FF; stw r6, 0x84(r31);
            }),
        )?;
    } else if version == Version::NtscJ {
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("__ct__14CSystemOptionsFv", version) + 0x1bc, {
                li r6, 100; stw r6, 0x664(r31); lis r6, 0xF7FF; stw r6, 0x668(r31);
            }),
        )?;
    } else {
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("__ct__14CSystemOptionsFv", version) + 0x194, {
                li r6, 100; stw r6, 0xcc(r3); lis r6, 0xF7FF; stw r6, 0xd0(r3);
            }),
        )?;
    }

    if version == Version::Pal {
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("__ct__14CSystemOptionsFRC12CInputStream", version) + 0x330, {
            li r6, 100; stw r6, 0x80(r28); lis r6, 0xF7FF; stw r6, 0x84(r28); mr r3, r29; li r4, 2;
        }))?;
    } else if version == Version::NtscJ {
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("__ct__14CSystemOptionsFRC12CInputStream", version) + 0x310, {
            li r6, 100; stw r6, 0x664(r29); lis r6, 0xF7FF; stw r6, 0x668(r29); mr r3, r30; li r4, 2;
        }))?;
    } else {
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("__ct__14CSystemOptionsFRC12CInputStream", version) + 0x308, {
            li r6, 100; stw r6, 0xcc(r28); lis r6, 0xF7FF; stw r6, 0xd0(r28); mr r3, r29; li r4, 2;
        }))?;
    }

    match config.difficulty_behavior {
        DifficultyBehavior::NormalOnly => {
            let patch = ppcasm!(symbol_addr!("DoPopupAdvance__19SNewFileSelectFrameFPC14CGuiTableGroup", version) + 0x78, {
                b { symbol_addr!("DoPopupAdvance__19SNewFileSelectFrameFPC14CGuiTableGroup", version) + 0xd0 };
            });
            dol_patcher.ppcasm_patch(&patch)?;
        }
        DifficultyBehavior::HardOnly => {}
        DifficultyBehavior::Either => {
            let patch = ppcasm!(symbol_addr!("ActivateNewGamePopup__19SNewFileSelectFrameFv", version) + 0x3C, {
                li r4, 2;
            });
            dol_patcher.ppcasm_patch(&patch)?;
        }
    };

    if config.difficulty_behavior != DifficultyBehavior::Either {
        let only_one_option_jump_offset = if version == Version::Pal || version == Version::NtscJ {
            0x210
        } else {
            0x1f8
        };
        let only_one_option_patch = ppcasm!(symbol_addr!("ActivateNewGamePopup__19SNewFileSelectFrameFv", version) + 0x110, {
            b { symbol_addr!("ActivateNewGamePopup__19SNewFileSelectFrameFv", version) + only_one_option_jump_offset };
        });
        dol_patcher.ppcasm_patch(&only_one_option_patch)?;
    }

    if config.force_fusion {
        let patch = ppcasm!(symbol_addr!("GetIsFusionEnabled__12CPlayerStateFv", version) + 4, {
            li r0, 1;
        });
        dol_patcher.ppcasm_patch(&patch)?;
    }

    dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("ShouldSkipCinematic__22CScriptSpecialFunctionFR13CStateManager", version), {
        li r3, 0x1; blr;
    }))?;

    Ok(())
}

fn patch_game_start(
    dol_patcher: &mut DolPatcher<'_>,
    _emitter: &mut TextEmitter,
    version: Version,
    config: &PatchConfig,
    spawn_room: SpawnRoomData,
) -> Result<(), String> {
    let no_starting_beam = !config.starting_items.power_beam
        && !config.starting_items.wave
        && !config.starting_items.ice
        && !config.starting_items.plasma;

    // New-game spawn mid-transition into the starting visor. Not persisted: save
    // reloads and post-elevator respawns reset to combat and are corrected by the
    // visor auto-transition inventory gate, so only this site needs patching.
    if config.starting_visor != Visor::Combat {
        let visor = config.starting_visor as u16;
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("__ct__12CPlayerStateFv", version) + 0x68, {
                li r0, visor; stw r6, 0x14(r31); stw r0, 0x18(r31);
            }),
        )?;
    }

    // spawn with weapon holstered instead of drawn when no beam is owned
    if no_starting_beam {
        let patch_offset = if version == Version::Pal || version == Version::NtscJ {
            0x3bc
        } else {
            0x434
        };
        dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("__ct__7CPlayerF9TUniqueIdRC12CTransform4fRC6CAABoxUi9CVector3fffffRC13CMaterialList", version) + patch_offset, {
            li r0, 0;
        }))?;
    }

    dol_patcher.ppcasm_patch(
        &ppcasm!(symbol_addr!("__ct__12CPlayerStateFv", version) + 0x58, {
            li r0, {config.starting_beam as u16}; stw r0, 0x8(r31);
        }),
    )?;

    if version == Version::Pal || version == Version::NtscJ {
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x0c, {
                lis r3, {spawn_room.mlvl}@h;
            }),
        )?;
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x18, {
                addi r0, r3, {spawn_room.mlvl}@l;
            }),
        )?;
    } else {
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x04, {
                lis r4, {spawn_room.mlvl}@h;
            }),
        )?;
        dol_patcher.ppcasm_patch(
            &ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x10, {
                addi r0, r4, {spawn_room.mlvl}@l;
            }),
        )?;
    }

    dol_patcher.ppcasm_patch(
        &ppcasm!(symbol_addr!("__ct__11CWorldStateFUi", version) + 0x10, {
            li r0, { spawn_room.mrea_idx };
        }),
    )?;

    Ok(())
}

// Inventory-gate DOL patches: starting without a beam and/or visor behaves the same
// everywhere (fresh spawn, save reload, world/morph transitions). Every check is a
// no-op when the relevant item is owned, so all are applied unconditionally. The
// trampolines land in code caves (emit_and_patch); displaced first instructions are
// read from the target so they stay correct across versions.
fn patch_inventory_gates(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    let pal_like = version == Version::Pal || version == Version::NtscJ;
    let start_transition_to_visor = symbol_addr!(
        "StartTransitionToVisor__12CPlayerStateFQ212CPlayerState12EPlayerVisor",
        version
    );
    let draw_gun = symbol_addr!("DrawGun__7CPlayerFR13CStateManager", version);

    // Skip beam fire when the equipped beam isn't owned. Phazon swaps the gun without
    // changing x310_currentBeam, so exempt it via its active flag.
    let update_normal_shot_cycle = symbol_addr!(
        "UpdateNormalShotCycle__10CPlayerGunFfR13CStateManager",
        version
    );
    let orig = dol_patcher.read_u32(update_normal_shot_cycle)?;
    emitter.emit_and_patch(dol_patcher, update_normal_shot_cycle, false, |addr| {
        ppcasm!(addr, {
                // r3 = CPlayerGun*, r4 = CStateManager&
                lbz     r11, 0x833(r3);
                andi    r11, r11, 0x8;      // x833_28_phazonBeamActive
                cmplwi  r11, 0;
                bne     beam_owned;
                lwz     r11, 0x310(r3);     // x310_currentBeam
                cmplwi  r11, 3;
                bgt     beam_owned;
                lwz     r12, 0x8b8(r4);
                lwz     r12, 0x0(r12);      // CPlayerState*
                rlwinm  r11, r11, 3, 0, 28;
                add     r12, r12, r11;
                lwz     r11, 0x2c(r12);     // beam item capacity
                cmplwi  r11, 0;
                bne     beam_owned;
                li      r11, 0;
                stw     r11, 0x2f0(r3);     // suppress the gun recoil motion
                blr;                        // beam not owned; fire nothing
            beam_owned:
                .long orig;                 // displaced first instruction
                b       { update_normal_shot_cycle + 4 };
        })
        .encoded_bytes()
    })?;

    // A visorless player is technically in scan visor; report scanning as invalid
    // unless Scan Visor is owned.
    let validate_scanning = symbol_addr!(
        "ValidateScanning__7CPlayerCFRC11CFinalInputR13CStateManager",
        version
    );
    let orig = dol_patcher.read_u32(validate_scanning)?;
    emitter.emit_and_patch(dol_patcher, validate_scanning, false, |addr| {
        ppcasm!(addr, {
                // r3 = CPlayer*, r4 = CFinalInput&, r5 = CStateManager&
                lwz     r11, 0x8b8(r5);
                lwz     r11, 0x0(r11);      // CPlayerState*
                lwz     r11, 0x54(r11);     // scan visor item capacity
                cmplwi  r11, 0;
                bne     scan_visor_owned;
                li      r3, 0;
                blr;                        // scan visor not owned; nothing is scannable
            scan_visor_owned:
                .long orig;                 // displaced first instruction
                b       { validate_scanning + 4 };
        })
        .encoded_bytes()
    })?;

    // Morph forces a transition to combat visor; redirect to the best owned real visor
    // (combat > thermal > xray, else fake combat). Scan is never picked, so a
    // scan-only/visorless player unmorphs into the fake combat visor.
    let enter_morph_visor_callsite =
        symbol_addr!("EnterMorphBallState__7CPlayerFR13CStateManager", version)
            + if pal_like { 0xb8 } else { 0x110 };
    emitter.emit_and_patch(dol_patcher, enter_morph_visor_callsite, true, |addr| {
        ppcasm!(addr, {
                // r3 = CPlayerState*, r4 = kPV_Combat (0)
                lwz     r11, 0xb4(r3);      // combat visor item capacity
                cmplwi  r11, 0;
                bne     do_transition;
                lwz     r11, 0x74(r3);      // thermal visor item capacity
                cmplwi  r11, 0;
                li      r4, 3;              // thermal
                bne     do_transition;
                lwz     r11, 0x94(r3);      // xray visor item capacity
                cmplwi  r11, 0;
                li      r4, 1;              // xray
                bne     do_transition;
                li      r4, 0;              // fake combat visor
            do_transition:
                b       { start_transition_to_visor };
        })
        .encoded_bytes()
    })?;

    // Refuse to raise the gun with no beam owned or in/entering scan visor. Retail
    // never draws in scan visor, but a combat-visorless player unmorphing there now can.
    let orig = dol_patcher.read_u32(draw_gun)?;
    emitter.emit_and_patch(dol_patcher, draw_gun, false, |addr| {
        ppcasm!(addr, {
                // r3 = CPlayer*, r4 = CStateManager&
                lwz     r11, 0x8b8(r4);
                lwz     r11, 0x0(r11);      // CPlayerState*
                lwz     r12, 0x18(r11);     // transitioning visor
                cmplwi  r12, 2;             // scan
                beq     refuse_draw;
                lwz     r12, 0x2c(r11);     // power beam item capacity
                lwz     r0, 0x34(r11);      // ice beam item capacity
                or      r12, r12, r0;
                lwz     r0, 0x3c(r11);      // wave beam item capacity
                or      r12, r12, r0;
                lwz     r0, 0x44(r11);      // plasma beam item capacity
                or      r12, r12, r0;
                cmplwi  r12, 0;
                bne     beam_owned;
            refuse_draw:
                blr;                        // no beam owned or in scan visor; stay holstered
            beam_owned:
                .long orig;                 // displaced first instruction
                b       { draw_gun + 4 };
        })
        .encoded_bytes()
    })?;

    // Per-frame reconcile of gun and inventory: holster when no beam is owned, draw when
    // the equipped beam isn't owned but a real one is (so the gun's state machine can
    // switch beams - it only processes input while drawn). Holster/Draw no-op when
    // already in that state. CPlayer offsets shifted in later builds.
    let (morph_state_offset, gun_offset) =
        if [Version::NtscU0_02, Version::Pal, Version::NtscJ].contains(&version) {
            (0x308, 0x4a0)
        } else {
            (0x2f8, 0x490)
        };
    let update_gun_state = symbol_addr!(
        "UpdateGunState__7CPlayerFRC11CFinalInputR13CStateManager",
        version
    );
    let holster_gun = symbol_addr!("HolsterGun__7CPlayerFR13CStateManager", version);
    let orig = dol_patcher.read_u32(update_gun_state)?;
    emitter.emit_and_patch(dol_patcher, update_gun_state, false, |addr| {
        ppcasm!(addr, {
                // r3 = CPlayer*, r4 = CFinalInput&, r5 = CStateManager&
                lwz     r11, 0x8b8(r5);
                lwz     r11, 0x0(r11);      // CPlayerState*
                lwz     r12, 0x2c(r11);     // power beam item capacity
                lwz     r0, 0x34(r11);      // ice beam item capacity
                or      r12, r12, r0;
                lwz     r0, 0x3c(r11);      // wave beam item capacity
                or      r12, r12, r0;
                lwz     r0, 0x44(r11);      // plasma beam item capacity
                or      r12, r12, r0;
                cmplwi  r12, 0;
                bne     beam_owned;
            force_holster:
                // no beam owned: force holster
                stwu    r1, -0x20(r1);
                mflr    r0;
                stw     r0, 0x24(r1);
                stw     r3, 0x8(r1);
                stw     r4, 0xc(r1);
                stw     r5, 0x10(r1);
                mr      r4, r5;
                bl      { holster_gun };
                lwz     r3, 0x8(r1);
                lwz     r4, 0xc(r1);
                lwz     r5, 0x10(r1);
                lwz     r0, 0x24(r1);
                mtlr    r0;
                addi    r1, r1, 0x20;
                b       run_function;
            beam_owned:
                lwz     r12, { morph_state_offset }(r3);
                cmplwi  r12, 0;             // kMS_Unmorphed
                bne     run_function;
                // never keep the gun out in or entering scan visor
                lwz     r12, 0x18(r11);     // transitioning visor
                cmplwi  r12, 2;             // scan
                beq     force_holster;
                lwz     r12, { gun_offset }(r3);
                lwz     r12, 0x310(r12);    // equipped beam
                cmplwi  r12, 3;
                bgt     run_function;
                rlwinm  r12, r12, 3, 0, 28;
                add     r12, r11, r12;
                lwz     r12, 0x2c(r12);     // equipped beam item capacity
                cmplwi  r12, 0;
                bne     run_function;       // equipped beam is owned; nothing to fix
                stwu    r1, -0x20(r1);
                mflr    r0;
                stw     r0, 0x24(r1);
                stw     r3, 0x8(r1);
                stw     r4, 0xc(r1);
                stw     r5, 0x10(r1);
                mr      r4, r5;
                bl      { draw_gun };
                lwz     r3, 0x8(r1);
                lwz     r4, 0xc(r1);
                lwz     r5, 0x10(r1);
                lwz     r0, 0x24(r1);
                mtlr    r0;
                addi    r1, r1, 0x20;
            run_function:
                .long orig;                 // displaced first instruction
                b       { update_gun_state + 4 };
        })
        .encoded_bytes()
    })?;

    // When no beam is input-selected and the equipped one isn't owned, auto-equip the
    // first owned beam via HandleBeamChange (reuses retail anim/sfx/ammo).
    let handle_beam_change = symbol_addr!(
        "HandleBeamChange__10CPlayerGunFRC11CFinalInputR13CStateManager",
        version
    );
    let auto_equip_callsite = handle_beam_change + 0xa0;
    let orig = dol_patcher.read_u32(auto_equip_callsite)?;
    emitter.emit_and_patch(dol_patcher, auto_equip_callsite, false, |addr| {
        ppcasm!(addr, {
                // r26 = input-selected beam (-1 = none), r29 = CPlayerState*, r30 = CPlayerGun*
                cmpwi   r26, -1;
                bne     done;
                lwz     r11, 0x310(r30);    // equipped beam
                cmplwi  r11, 3;
                bgt     done;
                rlwinm  r11, r11, 3, 0, 28;
                add     r11, r29, r11;
                lwz     r11, 0x2c(r11);     // equipped beam item capacity
                cmplwi  r11, 0;
                bne     done;               // equipped beam is owned; nothing to fix
                // select the first owned beam, if any
                lwz     r11, 0x2c(r29);
                cmplwi  r11, 0;
                li      r26, 0;             // power
                bne     done;
                lwz     r11, 0x34(r29);
                cmplwi  r11, 0;
                li      r26, 1;             // ice
                bne     done;
                lwz     r11, 0x3c(r29);
                cmplwi  r11, 0;
                li      r26, 2;             // wave
                bne     done;
                lwz     r11, 0x44(r29);
                cmplwi  r11, 0;
                li      r26, 3;             // plasma
                bne     done;
                li      r26, -1;            // no beam owned
            done:
                .long orig;                 // displaced instruction (cmpwi r26, -1)
                b       { handle_beam_change + 0xa4 };
        })
        .encoded_bytes()
    })?;

    // When the current visor isn't owned, transition to the first owned real visor
    // (combat > thermal > xray). Injected in UpdateVisorState's gated block so retail
    // switch conditions apply. Scan is never auto-equipped (it holsters the gun); the
    // player picks it manually. PAL/JPN use different registers/offsets.
    let update_visor_state = symbol_addr!(
        "UpdateVisorState__7CPlayerFRC11CFinalInputfR13CStateManager",
        version
    );
    let auto_trans_callsite = update_visor_state + if pal_like { 0xa4 } else { 0xb0 };
    let auto_trans_resume = update_visor_state + if pal_like { 0xa8 } else { 0xb4 };
    let orig = dol_patcher.read_u32(auto_trans_callsite)?;
    if pal_like {
        emitter.emit_and_patch(dol_patcher, auto_trans_callsite, false, |addr| {
            ppcasm!(addr, {
                    // r28 = CPlayerState*, r29 = CPlayer*, r31 = CStateManager&
                    // map current visor (0 combat, 1 xray, 2 scan, 3 thermal) to capacity offset
                    lwz     r11, 0x14(r28);     // current visor
                    cmplwi  r11, 0;
                    li      r12, 0xb4;          // combat
                    beq     have_offset;
                    cmplwi  r11, 1;
                    li      r12, 0x94;          // xray
                    beq     have_offset;
                    cmplwi  r11, 2;
                    li      r12, 0x54;          // scan
                    beq     have_offset;
                    li      r12, 0x74;          // thermal
                have_offset:
                    lwzx    r11, r28, r12;      // current visor item capacity
                    cmplwi  r11, 0;
                    bne     run_block;
                    // select the first owned real visor, if any
                    lwz     r11, 0xb4(r28);     // combat visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 0;
                    bne     transition;
                    lwz     r11, 0x74(r28);     // thermal visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 3;
                    bne     transition;
                    lwz     r11, 0x94(r28);     // xray visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 1;
                    bne     transition;
                    b       run_block;          // no real visor owned
                transition:
                    mr      r3, r28;
                    mr      r4, r12;
                    bl      { start_transition_to_visor };
                    // retail draws the gun on a visor switch (never scan here)
                    mr      r3, r29;
                    mr      r4, r31;
                    bl      { draw_gun };
                run_block:
                    .long orig;                 // displaced instruction
                    b       { auto_trans_resume };
            })
            .encoded_bytes()
        })?;
    } else {
        emitter.emit_and_patch(dol_patcher, auto_trans_callsite, false, |addr| {
            ppcasm!(addr, {
                    // r28 = CPlayer*, r30 = CStateManager&, r31 = CPlayerState*
                    // map current visor (0 combat, 1 xray, 2 scan, 3 thermal) to capacity offset
                    lwz     r11, 0x14(r31);     // current visor
                    cmplwi  r11, 0;
                    li      r12, 0xb4;          // combat
                    beq     have_offset;
                    cmplwi  r11, 1;
                    li      r12, 0x94;          // xray
                    beq     have_offset;
                    cmplwi  r11, 2;
                    li      r12, 0x54;          // scan
                    beq     have_offset;
                    li      r12, 0x74;          // thermal
                have_offset:
                    lwzx    r11, r31, r12;      // current visor item capacity
                    cmplwi  r11, 0;
                    bne     run_block;
                    // select the first owned real visor, if any
                    lwz     r11, 0xb4(r31);     // combat visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 0;
                    bne     transition;
                    lwz     r11, 0x74(r31);     // thermal visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 3;
                    bne     transition;
                    lwz     r11, 0x94(r31);     // xray visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 1;
                    bne     transition;
                    b       run_block;          // no real visor owned
                transition:
                    mr      r3, r31;
                    mr      r4, r12;
                    bl      { start_transition_to_visor };
                    // retail draws the gun on a visor switch (never scan here)
                    mr      r3, r28;
                    mr      r4, r30;
                    bl      { draw_gun };
                run_block:
                    .long orig;                 // displaced instruction
                    b       { auto_trans_resume };
            })
            .encoded_bytes()
        })?;
    }

    // Exiting scan visor via fire/missile: retail requires Combat Visor. Replace that
    // HasPowerUp call - transition to the best owned visor (else fake combat), draw the
    // gun, and report "not owned" to skip the retail block. PAL/JPN use different
    // registers/offsets.
    let scan_exit_callsite = update_visor_state + if pal_like { 0xe0 } else { 0xec };
    if pal_like {
        emitter.emit_and_patch(dol_patcher, scan_exit_callsite, true, |addr| {
            ppcasm!(addr, {
                    // bl-callee; r3 = CPlayerState*, r4 = combat visor item (ignored)
                    // live caller registers: r29 = CPlayer*, r31 = CStateManager&
                    lwz     r11, 0xb4(r3);      // combat visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 0;             // combat
                    bne     transition;
                    lwz     r11, 0x74(r3);      // thermal visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 3;             // thermal
                    bne     transition;
                    lwz     r11, 0x94(r3);      // xray visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 1;             // xray
                    bne     transition;
                    // nothing owned: fake combat visor so the gun stays reachable
                    li      r12, 0;
                transition:
                    stwu    r1, -0x10(r1);
                    mflr    r0;
                    stw     r0, 0x14(r1);
                    mr      r4, r12;
                    bl      { start_transition_to_visor };
                    mr      r3, r29;
                    mr      r4, r31;
                    bl      { draw_gun };
                    lwz     r0, 0x14(r1);
                    mtlr    r0;
                    addi    r1, r1, 0x10;
                    li      r3, 0;              // already handled; skip the retail block
                    blr;
            })
            .encoded_bytes()
        })?;
    } else {
        emitter.emit_and_patch(dol_patcher, scan_exit_callsite, true, |addr| {
            ppcasm!(addr, {
                    // bl-callee; r3 = CPlayerState*, r4 = combat visor item (ignored)
                    // live caller registers: r28 = CPlayer*, r30 = CStateManager&
                    lwz     r11, 0xb4(r3);      // combat visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 0;             // combat
                    bne     transition;
                    lwz     r11, 0x74(r3);      // thermal visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 3;             // thermal
                    bne     transition;
                    lwz     r11, 0x94(r3);      // xray visor item capacity
                    cmplwi  r11, 0;
                    li      r12, 1;             // xray
                    bne     transition;
                    // nothing owned: fake combat visor so the gun stays reachable
                    li      r12, 0;
                transition:
                    stwu    r1, -0x10(r1);
                    mflr    r0;
                    stw     r0, 0x14(r1);
                    mr      r4, r12;
                    bl      { start_transition_to_visor };
                    mr      r3, r28;
                    mr      r4, r30;
                    bl      { draw_gun };
                    lwz     r0, 0x14(r1);
                    mtlr    r0;
                    addi    r1, r1, 0x10;
                    li      r3, 0;              // already handled; skip the retail block
                    blr;
            })
            .encoded_bytes()
        })?;
    }

    // In the fake combat visor, report HUD state None so retail tears down the combat
    // HUD. The permanent visor/beam menus and radar are only hidden by state transitions
    // that never run when booting straight into None, so hide them each None frame. Wraps
    // GetDesiredHudState's only callsite. CSamusHud offsets shift in PAL/JPN.
    let (visor_menu_offset, beam_menu_offset, radar_offset) = if pal_like {
        (0x2b0, 0x2b4, 0x2b8)
    } else {
        (0x2a4, 0x2a8, 0x2ac)
    };
    let hud_state_callsite = symbol_addr!(
        "UpdateStateTransition__9CSamusHudFfRC13CStateManager",
        version
    ) + if pal_like { 0x40 } else { 0x44 };
    let get_desired_hud_state =
        symbol_addr!("GetDesiredHudState__9CSamusHudCFRC13CStateManager", version);
    let set_visible_menu = symbol_addr!("SetIsVisibleGame__17CHudVisorBeamMenuFb", version);
    let set_visible_radar = symbol_addr!("SetIsVisibleGame__18CHudRadarInterfaceFb", version);
    emitter.emit_and_patch(dol_patcher, hud_state_callsite, true, |addr| {
        ppcasm!(addr, {
                // r3 = CSamusHud*, r4 = CStateManager&
                stwu    r1, -0x10(r1);
                mflr    r0;
                stw     r0, 0x14(r1);
                stw     r3, 0xc(r1);
                stw     r4, 0x8(r1);
                bl      { get_desired_hud_state };
                lwz     r4, 0x8(r1);
                cmplwi  r3, 0;              // EHudState::Combat
                bne     keep_state;
                lwz     r11, 0x8b8(r4);
                lwz     r11, 0x0(r11);      // CPlayerState*
                lwz     r12, 0xb4(r11);     // combat visor item capacity
                cmplwi  r12, 0;
                bne     keep_state;
                // null-check each: the CSamusHud ctor runs this before creating them
                lwz     r11, 0xc(r1);       // CSamusHud*
                lwz     r3, { visor_menu_offset }(r11);
                cmplwi  r3, 0;
                beq     skip_visor_menu;
                li      r4, 0;
                bl      { set_visible_menu };
            skip_visor_menu:
                lwz     r11, 0xc(r1);
                lwz     r3, { beam_menu_offset }(r11);
                cmplwi  r3, 0;
                beq     skip_beam_menu;
                li      r4, 0;
                bl      { set_visible_menu };
            skip_beam_menu:
                lwz     r11, 0xc(r1);
                lwz     r3, { radar_offset }(r11);
                cmplwi  r3, 0;
                beq     hud_hidden;
                li      r4, 0;
                bl      { set_visible_radar };
            hud_hidden:
                li      r3, 5;              // EHudState::None
            keep_state:
                lwz     r0, 0x14(r1);
                mtlr    r0;
                addi    r1, r1, 0x10;
                blr;
        })
        .encoded_bytes()
    })?;

    // Hide the targeting reticle in the fake combat visor.
    let target_reticle_draw = symbol_addr!(
        "Draw__22CCompoundTargetReticleCFRC13CStateManagerb",
        version
    );
    let orig = dol_patcher.read_u32(target_reticle_draw)?;
    emitter.emit_and_patch(dol_patcher, target_reticle_draw, false, |addr| {
        ppcasm!(addr, {
                // r3 = CCompoundTargetReticle*, r4 = CStateManager&, r5 = hideLockon
                lwz     r11, 0x8b8(r4);
                lwz     r11, 0x0(r11);      // CPlayerState*
                lwz     r12, 0x14(r11);     // current visor
                cmplwi  r12, 0;             // combat
                bne     visor_owned;
                lwz     r12, 0xb4(r11);     // combat visor item capacity
                cmplwi  r12, 0;
                bne     visor_owned;
                blr;                        // fake combat visor; draw no reticle
            visor_owned:
                .long orig;                 // displaced first instruction
                b       { target_reticle_draw + 4 };
        })
        .encoded_bytes()
    })?;

    // Refuse combat lock-on in the fake combat visor by adding an ownership test to
    // ValidateOrbitTargetId's combat-targetable check. Grapple points stay exempt so a
    // visorless player can still grapple.
    let validate_orbit_target_id = symbol_addr!(
        "ValidateOrbitTargetId__7CPlayerCF9TUniqueIdR13CStateManager",
        version
    );
    let (orbit_check_offset, orbit_resume_offset, orbit_block_offset) = if pal_like {
        (0x1e8, 0x1ec, 0x1f0)
    } else {
        (0x1f4, 0x1f8, 0x1fc)
    };
    let orbit_callsite = validate_orbit_target_id + orbit_check_offset;
    let orbit_resume = validate_orbit_target_id + orbit_resume_offset;
    let orbit_block = validate_orbit_target_id + orbit_block_offset;
    let tcast_grapple = symbol_addr!(
        "__ct__33TCastToPtr<19CScriptGrapplePoint>FP7CEntity",
        version
    );
    let orig = dol_patcher.read_u32(orbit_callsite)?;
    emitter.emit_and_patch(dol_patcher, orbit_callsite, false, |addr| {
        ppcasm!(addr, {
                // r3 = targetable visor flags, r4 = CPlayerState*, r31 = actor;
                // reached only when the current visor is combat
                lwz     r0, 0xb4(r4);       // combat visor item capacity
                cmplwi  r0, 0;
                beq     combat_unowned;
            run_flag_test:
                .long orig;                 // displaced combat targetable flag test
                b       { orbit_resume };
            combat_unowned:
                stwu    r1, -0x20(r1);
                mflr    r0;
                stw     r0, 0x24(r1);
                stw     r3, 0x8(r1);
                stw     r4, 0xc(r1);
                mr      r4, r31;
                addi    r3, r1, 0x10;       // 8 byte TCastToPtr temp
                bl      { tcast_grapple };
                lwz     r12, 0x4(r3);       // cast result
                lwz     r3, 0x8(r1);
                lwz     r4, 0xc(r1);
                lwz     r0, 0x24(r1);
                mtlr    r0;
                addi    r1, r1, 0x20;
                cmplwi  r12, 0;
                bne     run_flag_test;      // grapple point: retail validation
                b       { orbit_block };    // PlayerNotReadyToTarget
        })
        .encoded_bytes()
    })?;

    // The fake combat visor lives in the None state, which retail skips entirely:
    // re-enable UpdateEnergy/UpdateFreeLook (freelook sfx) and the base HUD frame
    // (hudmemos, escape timer). Per-visor interfaces are null in None, so the rest stays
    // hidden.
    let hud_update_gate = symbol_addr!("Update__9CSamusHudFfRC13CStateManagerUibb", version)
        + if pal_like { 0x380 } else { 0x35c };
    dol_patcher.ppcasm_patch(&ppcasm!(hud_update_gate, {
        nop; // was: beq past UpdateEnergy/UpdateFreeLook
    }))?;
    dol_patcher.ppcasm_patch(
        &ppcasm!(symbol_addr!("Draw__9CSamusHudCFRC13CStateManagerfUibb", version) + 0x30, {
                cmpwi   r0, 6;              // was: cmpwi r0, 5 (None); never matches now
        }),
    )?;

    // Hide the mini automapper in the fake combat visor. Retargets the minimap
    // GetVisorTransitionFactor call: owned -> retail function, else jump to the retail
    // zero-alpha path.
    let in_game_gui_draw = symbol_addr!("Draw__17CInGameGuiManagerCFRC13CStateManager", version);
    let minimap_gvtf_offset = if pal_like { 0x544 } else { 0x4a0 };
    let minimap_callsite = in_game_gui_draw + minimap_gvtf_offset;
    let minimap_zero_alpha = in_game_gui_draw + minimap_gvtf_offset + 0xc;
    let get_visor_transition_factor =
        symbol_addr!("GetVisorTransitionFactor__12CPlayerStateCFv", version);
    emitter.emit_and_patch(dol_patcher, minimap_callsite, true, |addr| {
        ppcasm!(addr, {
                // r3 = CPlayerState*; reached only when the current visor is combat
                lwz     r11, 0xb4(r3);      // combat visor item capacity
                cmplwi  r11, 0;
                beq     fake_combat_visor;
                b       { get_visor_transition_factor };
            fake_combat_visor:
                b       { minimap_zero_alpha }; // zero alpha path
        })
        .encoded_bytes()
    })?;

    Ok(())
}

// ============================================================================
//  RANDOMPRIME SAVE-SLOT PERSISTENT DATA - SCHEMA (single source of truth)
// ============================================================================
// randomprime stores its own data in CGameState's dead FRONT region. Its first serialized member
// is a hard-coded 128-byte array (count at CGameState+0x0, inline data at +0x4), written as the
// first 128 bytes of every save regardless of content. PutTo writes it and the load ctor reads it
// back; nothing else touches it (verified against the NTSC 0-00 disassembly and metaforce), so we
// overwrite it with our schema. (A tail placement was tried first but rich saves can serialize
// almost to the buffer end and clobber it - see patch_save_overflow_guard.)
//
// The constants below are the single source of truth: build_save_schema_block and the PPC
// trampolines (stamp / block / name) all derive their offsets from them, and the compile-time
// asserts fail the build if the block stops fitting. See doc/dol-patching.md.
//
//   off  size  field      notes
//   0    4     magic      SAVE_SCHEMA_MAGIC ("RPSV"); absent => not our save
//   4    4     version    SAVE_SCHEMA_VERSION (current = 1)
//   8    16    uuid       instance id; all-zero default
//   24   36    save_name  big-endian UTF-16, null-terminated (<= 17 chars)
//   60   68    (unused)   free front-region tail; future fields grow the block here (bump version)
//
// Reads use fixed offsets independent of CPlayerState width / itemMaxCapacity, so instances with
// different configs read each other's data. PutTo entry stamps the block (patch_save_uuid_stamp),
// StartGame gates loads on magic+uuid (patch_save_uuid_block), and the file-select renders
// save_name (patch_save_name).

// Smallest save buffer across versions (NTSC-U / K = 0x3ac = 940; PAL and NTSC-J over-allocate).
// Hard cap the runtime overflow guard (patch_save_overflow_guard) checks total content against.
const SAVE_BUFFER_SIZE_MIN: i32 = 0x3ac; // 940

// CGameState's dead front region. Referenced only from SAVE_FRONT_UNUSED / the asserts below
#[allow(dead_code)]
const SAVE_FRONT_SIZE: i32 = 128;
// CGameState member holding the front region's inline bytes.
const SAVE_FRONT_DATA_MEMBER_OFF: i32 = 0x4;

const SAVE_SCHEMA_SIZE: i32 = 60; // magic + version + uuid + save_name
const SAVE_SCHEMA_OFFSET: i32 = 0; // == buffer offset; front starts at 0

#[allow(dead_code)]
const SAVE_FRONT_UNUSED: i32 = SAVE_FRONT_SIZE - (SAVE_SCHEMA_OFFSET + SAVE_SCHEMA_SIZE); // 68

const SAVE_SCHEMA_MAGIC: u32 = 0x5250_5356; // "RPSV"
const SAVE_SCHEMA_VERSION: u32 = 1;

// Field offsets within the block.
const SAVE_F_MAGIC: usize = 0;
const SAVE_F_VERSION: usize = 4;
const SAVE_F_UUID: usize = 8;
const SAVE_F_NAME: usize = 24;

// Absolute buffer offsets (block start + field), used by the trampolines.
const SAVE_OFF_MAGIC: i32 = SAVE_SCHEMA_OFFSET + SAVE_F_MAGIC as i32;
const SAVE_OFF_UUID: i32 = SAVE_SCHEMA_OFFSET + SAVE_F_UUID as i32;
const SAVE_OFF_NAME: i32 = SAVE_SCHEMA_OFFSET + SAVE_F_NAME as i32;

const UUID_BYTES: usize = 16;

// ASCII-only; 17 chars + null = 18 UTF-16 units = 9 words (2 big-endian units per word).
const SAVE_NAME_WORDS: usize = 9;
const SAVE_NAME_MAX_CHARS: usize = 17;

// Scratch stride per file-select row; must be a power of 2 and >= SAVE_NAME_WORDS*4 for the
// slwi shift-6 row offset. 9*4=36 rounds up to 64 (2^6); the padding per row is never accessed.
const SAVE_NAME_SCRATCH_STRIDE: usize = 64;
// File-select rows. Each row's name is reassembled into its OWN scratch slot because wstring_l /
// SetText store the pointer rather than copying, so a shared buffer would alias across rows.
const SAVE_NAME_ROWS: usize = 3;

const MNUMWRITES_OFF: i32 = 0x10; // COutputStream::mNumWrites (bytes flushed)

const _: () = assert!(
    SAVE_FRONT_UNUSED >= 0,
    "save schema block overflows CGameState's dead front region"
);
const _: () = assert!(
    SAVE_F_NAME + SAVE_NAME_WORDS * 4 <= SAVE_SCHEMA_SIZE as usize,
    "save_name field overflows the schema block"
);
const _: () = assert!(
    SAVE_F_UUID + UUID_BYTES <= SAVE_F_NAME,
    "uuid field overlaps save_name"
);
const _: () = assert!((SAVE_NAME_MAX_CHARS + 1) <= SAVE_NAME_WORDS * 2);
const _: () =
    assert!(SAVE_NAME_WORDS * 4 <= SAVE_NAME_SCRATCH_STRIDE && SAVE_NAME_SCRATCH_STRIDE == 64);

// Encode a saveName: null-terminated, zero-padded big-endian UTF-16 (two units per word).
// Non-ASCII characters become '?'.
fn build_save_name_words(save_name: &str) -> Vec<u8> {
    let mut units: Vec<u16> = save_name
        .chars()
        .take(SAVE_NAME_MAX_CHARS)
        .map(|ch| {
            if ch.is_ascii() {
                ch as u16
            } else {
                b'?' as u16
            }
        })
        .collect();
    units.push(0); // null terminator
    units.resize(SAVE_NAME_WORDS * 2, 0); // zero-pad (and bound) to 32 code units
    let mut bytes = Vec::with_capacity(SAVE_NAME_WORDS * 4);
    for unit in units {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    bytes
}

// Per-version struct/instruction offsets for the save-uuid/name feature, read from each build's
// disassembly; localized builds (PAL, NTSC-J) shift CGameState members off the NTSC-U family.
struct SaveUuidLayout {
    start_hook_off: u32,     // StartGame: the `bl BuildNewFileSlot` load (block hook)
    start_epilogue_off: u32, // StartGame: epilogue (enter-game store skipped on deny)
    driver_off: i32,         // CSaveGameScreen -> CMemoryCardDriver (block)
    slot_table_off: i32,     // driver -> SGameFileSlot table (block)
    slot_buf_off: i32,       // SGameFileSlot -> raw save buffer (block)
    name_buf_off: i32,       // SetupFrameContents: GameFileStateInfo reg -> raw save buffer (name)
    setup_hook_off: u32,     // SetupFrameContents hook (`cmplwi <name>, 0`)
    // false = NTSC/PAL (name r25, fileInfo r26); true = NTSC-J (shifted up one: r26 / r27).
    setup_regs_shifted: bool,
}

fn save_uuid_layout(version: Version) -> Option<SaveUuidLayout> {
    match version {
        // NTSC-U family + Korean share a byte-identical CGameState/CSaveGameScreen layout.
        Version::NtscU0_00 | Version::NtscU0_01 | Version::NtscU0_02 | Version::NtscK => {
            Some(SaveUuidLayout {
                start_hook_off: 0x40,
                start_epilogue_off: 0x60,
                driver_off: 0x6c,
                slot_table_off: 0xec,
                slot_buf_off: 0x4,
                name_buf_off: -0x3ac,
                setup_hook_off: 0x200,
                setup_regs_shifted: false,
            })
        }
        // PAL: larger save buffer (fileInfo at slot+0xa88 -> name_buf_off = 0x4 - 0xa88 = -0xa84);
        // NTSC register allocation.
        Version::Pal => Some(SaveUuidLayout {
            start_hook_off: 0x40,
            start_epilogue_off: 0x60,
            driver_off: 0x6c,
            slot_table_off: 0xb0,
            slot_buf_off: 0x4,
            name_buf_off: -0xa84,
            setup_hook_off: 0x200,
            setup_regs_shifted: false,
        }),
        // NTSC-J: largest shifts (slot table 0x694; fileInfo at slot+0x890 -> name_buf_off = -0x88c);
        // SetupFrameContents saves one fewer non-volatile (regs shifted up one, hook at +0x1fc).
        Version::NtscJ => Some(SaveUuidLayout {
            start_hook_off: 0x40,
            start_epilogue_off: 0x60,
            driver_off: 0x6c,
            slot_table_off: 0x694,
            slot_buf_off: 0x4,
            name_buf_off: -0x88c,
            setup_hook_off: 0x1fc,
            setup_regs_shifted: true,
        }),
        _ => None,
    }
}

// Must-fit cave data for the save-uuid/name feature. emit_addressed has no overflow path (panics if
// no cave fits), so these are reserved early to guarantee a slot; the feature's trampolines emit
// later and overflow to the heap safely. See patch_dol.
struct SaveUuidData {
    block_addr: u32, // the SAVE_SCHEMA_SIZE-byte schema block (magic+version+uuid+name+reserved)
    scratch_addr: u32, // per-row name-reassembly buffers (patch_save_name)
}

// Build the SAVE_SCHEMA_SIZE-byte schema block exactly as it lives in the front region.
fn build_save_schema_block(config: &PatchConfig) -> Vec<u8> {
    let mut block = vec![0u8; SAVE_SCHEMA_SIZE as usize];
    block[SAVE_F_MAGIC..][..4].copy_from_slice(&SAVE_SCHEMA_MAGIC.to_be_bytes());
    block[SAVE_F_VERSION..][..4].copy_from_slice(&SAVE_SCHEMA_VERSION.to_be_bytes());
    block[SAVE_F_UUID..][..UUID_BYTES].copy_from_slice(&config.uuid);
    let name_bytes = config
        .save_name
        .as_deref()
        .map(build_save_name_words)
        .unwrap_or_else(|| vec![0u8; SAVE_NAME_WORDS * 4]);
    block[SAVE_F_NAME..][..name_bytes.len()].copy_from_slice(&name_bytes);
    block
}

// Reserve the feature's must-fit cave data. UUID is always present (defaulting to zeros), so the
// feature installs for every supported version; None only when the version is unsupported.
fn reserve_save_uuid_data(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    config: &PatchConfig,
) -> Result<Option<SaveUuidData>, String> {
    if save_uuid_layout(version).is_none() {
        return Ok(None);
    }

    let block = build_save_schema_block(config);
    let block_addr = emitter.emit_addressed(dol_patcher, move |_| block.clone())?;
    let scratch_addr = emitter.emit_addressed(dol_patcher, move |_| {
        vec![0u8; SAVE_NAME_SCRATCH_STRIDE * SAVE_NAME_ROWS]
    })?;

    Ok(Some(SaveUuidData {
        block_addr,
        scratch_addr,
    }))
}

// Stamp the schema block into CGameState's front-region member on PutTo entry, into the LIVE
// object (not the output buffer). PutTo's own unmodified serialization loop then writes those
// bytes to buffer offset 0 moments later.
fn patch_save_uuid_stamp(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    data: &Option<SaveUuidData>,
) -> Result<(), String> {
    let Some(data) = data else {
        return Ok(());
    };
    if save_uuid_layout(version).is_none() {
        return Ok(());
    }
    let Some(putto) = symbol_addr_opt!("PutTo__10CGameStateCFR13COutputStream", version) else {
        return Ok(());
    };

    let block_addr = data.block_addr;

    let putto_orig = dol_patcher.read_u32(putto)?;
    emitter.emit_and_patch(dol_patcher, putto, false, |cave_addr| {
        // r3 = CGameState* (this); use r6/r9/r12 as scratch.
        ppcasm!(cave_addr, {
            addi  r12, r3, { SAVE_FRONT_DATA_MEMBER_OFF + SAVE_SCHEMA_OFFSET }; // front-region base
            lis   r9, { (block_addr >> 16) as i32 };
            ori   r9, r9, { (block_addr & 0xffff) as i32 };
            li    r6, { SAVE_SCHEMA_SIZE / 4 }; // word count
        stamp_loop:
            lwz   r0, 0x0(r9);
            stw   r0, 0x0(r12);
            addi  r9, r9, 4;
            addi  r12, r12, 4;
            addi  r6, r6, -1;
            cmpwi r6, 0;
            bne   stamp_loop;
            .long putto_orig;        // displaced prologue
            b     { putto + 4 };
        })
        .encoded_bytes()
    })?;

    Ok(())
}

// Block loading a save whose stamped UUID does not match this instance's. Hook StartGame's
// bl BuildNewFileSlot: check our magic at save_buf+SAVE_OFF_MAGIC (absent => not our save => allow),
// then compare the 4 UUID words; on mismatch skip the load, play a denied sfx, and branch to the
// epilogue (screen stays open). New/foreign/matching saves load normally; EraseGame is untouched.
fn patch_save_uuid_block(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    data: &Option<SaveUuidData>,
) -> Result<(), String> {
    let Some(data) = data else {
        return Ok(());
    };
    let Some(layout) = save_uuid_layout(version) else {
        return Ok(());
    };
    let (Some(start_game), Some(sfx_start), Some(build_slot)) = (
        symbol_addr_opt!("StartGame__15CSaveGameScreenFi", version),
        symbol_addr_opt!("SfxStart__11CSfxManagerFUsssbsbi", version),
        symbol_addr_opt!("BuildNewFileSlot__17CMemoryCardDriverFi", version),
    ) else {
        return Ok(());
    };
    let hook = start_game + layout.start_hook_off; // bl BuildNewFileSlot (the load)
    let continue_addr = hook + 4; // normal flow (sets enter-game)
    let epilogue = start_game + layout.start_epilogue_off; // epilogue (enter-game store skipped)

    let block_addr = data.block_addr; // magic at +SAVE_F_MAGIC, uuid at +SAVE_F_UUID
    let driver_off = layout.driver_off;
    let slot_table_off = layout.slot_table_off;
    let slot_buf_off = layout.slot_buf_off;

    emitter.emit_and_patch(dol_patcher, hook, false, |cave_addr| {
        ppcasm!(cave_addr, {
            cmpwi r31, 0;             // r31 = (GetGameFileStateInfo(idx) == null)
            bne   do_load;            // new file -> allow
            lwz   r11, { driver_off }(r29); // driver
            slwi  r12, r30, 3;        // idx * 8
            add   r12, r11, r12;
            lwz   r9, { slot_table_off }(r12); // SGameFileSlot*
            addi  r9, r9, { slot_buf_off }; // raw save buffer base
            lis   r10, { (block_addr >> 16) as i32 };
            ori   r10, r10, { (block_addr & 0xffff) as i32 };
            // magic absent => not our save => allow load
            lwz   r5, { SAVE_OFF_MAGIC }(r9);
            lwz   r11, { SAVE_F_MAGIC as i32 }(r10);
            cmplw r5, r11;
            bne   do_load;
            // compare 4 UUID words at save_buf+SAVE_OFF_UUID with block uuid
            lwz   r5, { SAVE_OFF_UUID }(r9);
            lwz   r6, { SAVE_OFF_UUID + 4 }(r9);
            lwz   r7, { SAVE_OFF_UUID + 8 }(r9);
            lwz   r8, { SAVE_OFF_UUID + 12 }(r9);
            lwz   r11, { SAVE_F_UUID as i32 }(r10);
            lwz   r12, { SAVE_F_UUID as i32 + 4 }(r10);
            cmplw r5, r11;
            bne   mismatch;
            cmplw r6, r12;
            bne   mismatch;
            lwz   r11, { SAVE_F_UUID as i32 + 8 }(r10);
            lwz   r12, { SAVE_F_UUID as i32 + 12 }(r10);
            cmplw r7, r11;
            bne   mismatch;
            cmplw r8, r12;
            bne   mismatch;
        do_load:
            // match or new file: run the original BuildNewFileSlot(driver, idx) and continue.
            lwz   r3, { driver_off }(r29);
            mr    r4, r30;
            bl    { build_slot };
            b     { continue_addr };
        mismatch:
            // foreign save: play denied sfx, stay on menu. SfxStart returns CSfxHandle by value in r3.
            addi  r3, r1, 0x8;        // CSfxHandle scratch (frame has 0x8..0x14 free)
            li    r4, 1094;           // SFXfnt_back
            li    r5, 0x7f;
            li    r6, 0x40;
            li    r7, 0;
            li    r8, 0x7f;           // priority
            li    r9, 0;
            li    r10, -1;            // kInvalidAreaId
            bl    { sfx_start };
            b     { epilogue };
        })
        .encoded_bytes()
    })?;

    Ok(())
}

// On each file-select row, show the stored saveName instead of the world name. Hook the
// `cmplwi <name>, 0` before SetupFrameContents builds the wstring: copy the 9 name words from
// fileInfo + name_buf_off + SAVE_OFF_NAME into per-row scratch and point `name` at it; if the first
// word is zero (no stored name), keep the world name. Row index is r30 in every build; name/fileInfo
// are r25/r26 on NTSC+PAL and r26/r27 on NTSC-J (setup_regs_shifted).
// allow(dead_code): ppcasm's per-label struct trips the lint through the local name_tramp! macro.
#[allow(dead_code)]
fn patch_save_name(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    data: &Option<SaveUuidData>,
) -> Result<(), String> {
    // Active whenever this build participates in the feature, so a build that omits saveName still
    // renders names other instances stamped.
    let Some(data) = data else {
        return Ok(());
    };
    let Some(layout) = save_uuid_layout(version) else {
        return Ok(());
    };
    let Some(setup) = symbol_addr_opt!("SetupFrameContents__19SNewFileSelectFrameFv", version)
    else {
        return Ok(());
    };
    let hook = setup + layout.setup_hook_off; // `cmplwi <name>, 0` before the wstring build
    let return_addr = hook + 4;

    let scratch_addr = data.scratch_addr;
    // Combined offsets from the fileInfo reg to the schema fields in the save buffer.
    let magic_off = layout.name_buf_off + SAVE_OFF_MAGIC;
    let name_off = layout.name_buf_off + SAVE_OFF_NAME;
    let regs_shifted = layout.setup_regs_shifted;

    emitter.emit_and_patch(dol_patcher, hook, false, |cave_addr| {
        // $name = world-name ptr reg, $fileinfo = GameFileStateInfo reg; scratch is r0/r3-r12 + r30.
        macro_rules! name_tramp {
            ($name:tt, $fileinfo:tt) => {
                ppcasm!(cave_addr, {
                    // r30 = row index; bound it so it can't write past the SAVE_NAME_ROWS scratch.
                    cmplwi r30, { SAVE_NAME_ROWS as i32 };
                    bge   keep_world;
                    // magic absent => not our save => keep the world name
                    addi  r9, $fileinfo, { magic_off };  // r9 = save_buf + SAVE_OFF_MAGIC
                    lwz   r0, 0x0(r9);
                    lis   r5, { (SAVE_SCHEMA_MAGIC >> 16) as i32 };
                    ori   r5, r5, { (SAVE_SCHEMA_MAGIC & 0xffff) as i32 };
                    cmplw r0, r5;
                    bne   keep_world;
                    slwi  r5, r30, 6;             // r30 * row_stride (= 64)
                    lis   r6, { (scratch_addr >> 16) as i32 };
                    ori   r6, r6, { (scratch_addr & 0xffff) as i32 };
                    add   r6, r6, r5;             // r6 = this row's scratch base (preserved for name ptr)
                    addi  r9, $fileinfo, { name_off }; // r9 = save_buf + SAVE_OFF_NAME
                    li    r5, { SAVE_NAME_WORDS as i32 };
                    mr    r12, r6;                // r12 = advancing scratch write ptr
                name_loop:
                    lwz   r0, 0x0(r9);
                    stw   r0, 0x0(r12);
                    addi  r9, r9, 4;
                    addi  r12, r12, 4;
                    addi  r5, r5, -1;
                    cmpwi r5, 0;
                    bne   name_loop;
                    lwz   r0, 0x0(r6);            // first name word (r6 = scratch base, preserved)
                    cmpwi r0, 0;
                    beq   keep_world;             // no stored name -> keep world name
                    mr    $name, r6;
                keep_world:
                    cmplwi $name, 0;              // displaced original
                    b     { return_addr };
                })
                .encoded_bytes()
            };
        }
        if regs_shifted {
            name_tramp!(r26, r27)
        } else {
            name_tramp!(r25, r26)
        }
    })?;

    Ok(())
}

// OSReport debug-logging hooks, gated behind the `osDiagnostics` preference. Add new
// diagnostic-only hooks here; safety-relevant guards go in patch_dol (see patch_save_overflow_guard).
fn patch_os_diagnostics(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    config: &PatchConfig,
) -> Result<(), String> {
    if !config.os_diagnostics {
        return Ok(());
    }
    patch_diag_resource_miss(dol_patcher, emitter, version)?;
    patch_diag_save_world_budget(dol_patcher, emitter, version)?;
    Ok(())
}

// Hard-coded MLVL asset ids (src/elevators.rs) for all 8 worlds CGameState::PutTo's per-world loop
// visits, in the same order the loop visits them (Area::EWorldIndex).
const WORLD_MLVL_END_CINEMA: u32 = 0x13D79165;
const WORLD_MLVL_FRIGATE: u32 = 0x158efe17;
const WORLD_MLVL_TALON: u32 = 0x39f2de28;
const WORLD_MLVL_MAGMOOR: u32 = 0x3ef8237c;
const WORLD_MLVL_CHOZO: u32 = 0x83f6ff6f;
const WORLD_MLVL_PHENDRANA: u32 = 0xa8be6291;
const WORLD_MLVL_MINES: u32 = 0xb1ac4d65;
const WORLD_MLVL_CRATER: u32 = 0xc13b09d1;

// Per-world save-size breakdown. Reports one actionable number per world, layer_bytes (the
// CWorldLayerState flag size, which scales with a config's own level edits), plus a global `free`
// (bytes left in the buffer). The other per-world fields are sized by fixed level geometry, so
// they aren't surfaced.
fn patch_diag_save_world_budget(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    if version != Version::NtscU0_00 {
        return Ok(());
    }
    let Some(putto) = symbol_addr_opt!("PutTo__10CGameStateCFR13COutputStream", version) else {
        return Ok(());
    };
    // Written by patch_diag_save_world_layer_bytes and read by patch_diag_save_world_row for the
    // same world, strictly sequentially, so it's always set before read and needs no reset.
    let layer_bytes_addr =
        emitter.emit_addressed(dol_patcher, move |_| 0u32.to_be_bytes().to_vec())?;

    patch_diag_save_world_baseline(dol_patcher, emitter, version, putto)?;
    patch_diag_save_world_layer_bytes(dol_patcher, emitter, version, layer_bytes_addr)?;
    patch_diag_save_world_row(dol_patcher, emitter, version, putto, layer_bytes_addr)?;
    Ok(())
}

// OSReports free space before any world is written. Hooks the per-world loop's initial branch to
// its condition check (PutTo + 0x11c, NTSC 0-00), which runs once per PutTo. Since the hooked
// instruction is a PC-relative branch (not replayable from the cave), the trampoline ends with a
// fresh branch to the loop's re-entry point instead of a displaced-instruction replay.
fn patch_diag_save_world_baseline(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    putto: u32,
) -> Result<(), String> {
    let Some(osreport) = symbol_addr_opt!("OSReport", version) else {
        return Ok(());
    };
    let hook = putto + 0x11c;
    let loop_cond = putto + 0x1a0;
    // Confirm we're hooking the unconditional branch we expect (opcode 18) before discarding it.
    let hook_orig = dol_patcher.read_u32(hook)?;
    if hook_orig & 0xfc00_0000 != 0x4800_0000 {
        return Err(format!(
            "patch_diag_save_world_baseline: expected an unconditional branch at PutTo+0x11c, found {hook_orig:#010x}"
        ));
    }

    emitter.emit_and_patch(dol_patcher, hook, false, |cave_addr| {
        ppcasm!(cave_addr, {
            b    skip;
            fmt: .asciiz b"randomprime: SAVE_SIZE_BASE free=%d\n";
        skip:
            lwz   r4, { MNUMWRITES_OFF }(r31); // bytes written before any world
            li    r5, { SAVE_BUFFER_SIZE_MIN };
            subf  r4, r4, r5;                  // free = limit - used
            lis   r3, fmt@h;
            addi  r3, r3, fmt@l;
            bl    { osreport };                // clobbers r0,r3-r12,LR,CTR
            b     { loop_cond };               // re-enter the loop where the displaced branch did
        })
        .encoded_bytes()
    })?;

    Ok(())
}

// Isolates one world's layer-flag cost by hooking CWorldState::PutTo's call to CWorldLayerState's
// PutTo (CWorldState::PutTo + 0x80, NTSC 0-00) and storing mNumWrites' delta across it, so
// `layer_bytes` excludes the fixed relay/area/door cost. Same PC-relative-branch handling as
// patch_diag_save_world_baseline; the original arg loads run before the hook, so r3/r4/r5 are live.
fn patch_diag_save_world_layer_bytes(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    layer_bytes_addr: u32,
) -> Result<(), String> {
    let (Some(worldstate_putto), Some(layerstate_putto)) = (
        symbol_addr_opt!("PutTo__11CWorldStateFR16CMemoryStreamOut", version),
        symbol_addr_opt!("PutTo__16CWorldLayerStateFR16CMemoryStreamOut", version),
    ) else {
        return Ok(());
    };
    let hook = worldstate_putto + 0x80;
    // Sanity-check we're replacing a `bl` (opcode 18, LK=1) before discarding its encoding.
    let hook_orig = dol_patcher.read_u32(hook)?;
    if hook_orig & 0xfc00_0003 != 0x4800_0001 {
        return Err(format!(
            "patch_diag_save_world_layer_bytes: expected `bl` at CWorldState::PutTo+0x80, found {hook_orig:#010x}"
        ));
    }

    emitter.emit_and_patch(dol_patcher, hook, false, |cave_addr| {
        ppcasm!(cave_addr, {
            // r3/r4/r5 are already PutTo__16CWorldLayerStateFR16CMemoryStreamOut's correct args
            // here (loaded by the untouched instructions immediately before this hook). r30 =
            // COutputStream*, per CWorldState::PutTo's own register allocation (mr r30,r4 at entry).
            lis   r9, { (layer_bytes_addr >> 16) as i32 };
            ori   r9, r9, { (layer_bytes_addr & 0xffff) as i32 };
            lwz   r8, { MNUMWRITES_OFF }(r30); // before
            stw   r8, 0x0(r9);                 // stash (overwritten with the real delta below)
            bl    { layerstate_putto };        // the real call; clobbers r0,r3-r12,LR,CTR
            lwz   r8, { MNUMWRITES_OFF }(r30); // after
            lis   r9, { (layer_bytes_addr >> 16) as i32 };
            ori   r9, r9, { (layer_bytes_addr & 0xffff) as i32 };
            lwz   r0, 0x0(r9);                 // before, reloaded (r9/r0 were clobbered by the call)
            subf  r0, r0, r8;                  // layer_bytes = after - before
            stw   r0, 0x0(r9);
            b     { hook + 4 };
        })
        .encoded_bytes()
    })?;

    Ok(())
}

// OSReports each world's name, layer_bytes, and free after its CWorldState::PutTo call (PutTo +
// 0x190, NTSC 0-00). r25 holds the world's MLVL id (non-volatile, survives the call) and r31 is
// the live CMemoryStreamOut*.
fn patch_diag_save_world_row(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    putto: u32,
    layer_bytes_addr: u32,
) -> Result<(), String> {
    let Some(osreport) = symbol_addr_opt!("OSReport", version) else {
        return Ok(());
    };
    let hook = putto + 0x190;
    let hook_orig = dol_patcher.read_u32(hook)?;

    emitter.emit_and_patch(dol_patcher, hook, false, |cave_addr| {
        ppcasm!(cave_addr, {
            b       skip;
            n_end_cinema: .asciiz b"End Cinema";
            n_frigate:    .asciiz b"Frigate";
            n_talon:      .asciiz b"Tallon";
            n_magmoor:    .asciiz b"Magmoor";
            n_chozo:      .asciiz b"Chozo";
            n_phendrana:  .asciiz b"Phendrana";
            n_mines:      .asciiz b"Mines";
            n_crater:     .asciiz b"Crater";
            fmt: .asciiz b"randomprime: SAVE_SIZE_WORLD %s - layer_bytes=%d, free=%d\n";
        skip:
            lis   r6, { (WORLD_MLVL_END_CINEMA >> 16) as i32 };
            ori   r6, r6, { (WORLD_MLVL_END_CINEMA & 0xffff) as i32 };
            cmplw r25, r6;
            bne   c_frigate;
            lis   r7, n_end_cinema@h;
            addi  r7, r7, n_end_cinema@l;
            b     report;
        c_frigate:
            lis   r6, { (WORLD_MLVL_FRIGATE >> 16) as i32 };
            ori   r6, r6, { (WORLD_MLVL_FRIGATE & 0xffff) as i32 };
            cmplw r25, r6;
            bne   c_talon;
            lis   r7, n_frigate@h;
            addi  r7, r7, n_frigate@l;
            b     report;
        c_talon:
            lis   r6, { (WORLD_MLVL_TALON >> 16) as i32 };
            ori   r6, r6, { (WORLD_MLVL_TALON & 0xffff) as i32 };
            cmplw r25, r6;
            bne   c_magmoor;
            lis   r7, n_talon@h;
            addi  r7, r7, n_talon@l;
            b     report;
        c_magmoor:
            lis   r6, { (WORLD_MLVL_MAGMOOR >> 16) as i32 };
            ori   r6, r6, { (WORLD_MLVL_MAGMOOR & 0xffff) as i32 };
            cmplw r25, r6;
            bne   c_chozo;
            lis   r7, n_magmoor@h;
            addi  r7, r7, n_magmoor@l;
            b     report;
        c_chozo:
            lis   r6, { (WORLD_MLVL_CHOZO >> 16) as i32 };
            ori   r6, r6, { (WORLD_MLVL_CHOZO & 0xffff) as i32 };
            cmplw r25, r6;
            bne   c_phendrana;
            lis   r7, n_chozo@h;
            addi  r7, r7, n_chozo@l;
            b     report;
        c_phendrana:
            lis   r6, { (WORLD_MLVL_PHENDRANA >> 16) as i32 };
            ori   r6, r6, { (WORLD_MLVL_PHENDRANA & 0xffff) as i32 };
            cmplw r25, r6;
            bne   c_mines;
            lis   r7, n_phendrana@h;
            addi  r7, r7, n_phendrana@l;
            b     report;
        c_mines:
            lis   r6, { (WORLD_MLVL_MINES >> 16) as i32 };
            ori   r6, r6, { (WORLD_MLVL_MINES & 0xffff) as i32 };
            cmplw r25, r6;
            bne   c_crater;
            lis   r7, n_mines@h;
            addi  r7, r7, n_mines@l;
            b     report;
        c_crater:
            lis   r6, { (WORLD_MLVL_CRATER >> 16) as i32 };
            ori   r6, r6, { (WORLD_MLVL_CRATER & 0xffff) as i32 };
            cmplw r25, r6;
            bne   done;
            lis   r7, n_crater@h;
            addi  r7, r7, n_crater@l;
        report:
            lis   r9, { (layer_bytes_addr >> 16) as i32 };
            ori   r9, r9, { (layer_bytes_addr & 0xffff) as i32 };
            lwz   r5, 0x0(r9);                 // layer_bytes (set moments ago for this world)
            lwz   r8, { MNUMWRITES_OFF }(r31); // used = cumulative bytes so far
            li    r6, { SAVE_BUFFER_SIZE_MIN };
            subf  r6, r8, r6;                  // free = limit - used
            mr    r4, r7;                      // name (%s)
            lis   r3, fmt@h;
            addi  r3, r3, fmt@l;
            bl    { osreport };                // clobbers r0,r3-r12,LR,CTR
        done:
            .long hook_orig;                   // displaced original instruction
            b     { hook + 4 };
        })
        .encoded_bytes()
    })?;

    Ok(())
}

// Detect (but not prevent) a save whose content overflows the engine's fixed buffer
// (SAVE_BUFFER_SIZE_MIN). This is a pre-existing engine limit, independent of our schema. Runs
// unconditionally, not gated on `osDiagnostics`: a silent overflow corrupts the save and surfaces
// later as a confusing crash on some subsequent load, so we panic at the moment it happens instead.
//
// Hooks PutTo's epilogue (the entry hook can't see the final size) to compare mNumWrites against
// the cap, OSReport the sizes, then fault on a recognizable sentinel address so the crash-screen
// handler reports it like any other crash.
//
// `verbose` (config.os_diagnostics) additionally OSReports SAVE_SIZE_TOTAL on every save, folded
// into this hook because emit_and_patch installs only one trampoline per site.
//
// NTSC 0-00 only: the epilogue offset was hand-verified against that build; other builds need the
// same verification first.
fn patch_save_overflow_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    verbose: bool,
) -> Result<(), String> {
    if version != Version::NtscU0_00 {
        return Ok(());
    }
    let (Some(putto), Some(osreport)) = (
        symbol_addr_opt!("PutTo__10CGameStateCFR13COutputStream", version),
        symbol_addr_opt!("OSReport", version),
    ) else {
        return Ok(());
    };
    // PutTo epilogue: `lmw r24, 0x30(r1)` restoring the non-volatiles. r31 = CMemoryStreamOut* is
    // still live here (restored to the same value by the displaced lmw). PutTo + 0x1b8 (NTSC 0-00).
    let epilogue = putto + 0x1b8;
    let epilogue_orig = dol_patcher.read_u32(epilogue)?;

    // Format strings are embedded inline in the stub (jumped over) rather than in a separate
    // must-fit cave, so the whole unconditional guard stays overflow-safe as one unit. `verbose`
    // picks which ppcasm! block to emit at patch time.
    emitter.emit_and_patch(dol_patcher, epilogue, false, |cave_addr| {
        if verbose {
            ppcasm!(cave_addr, {
                b     skip;
                fmt_total:    .asciiz b"randomprime: SAVE_SIZE_TOTAL free=%d\n";
                fmt_overflow: .asciiz b"randomprime: SAVE BUFFER OVERFLOW wrote=%08x limit=%08x\n";
            skip:
                lwz   r4, { MNUMWRITES_OFF }(r31); // used
                li    r0, { SAVE_BUFFER_SIZE_MIN };
                subf  r4, r4, r0;                   // r4 := free = limit - used
                lis   r3, fmt_total@h;
                addi  r3, r3, fmt_total@l;
                bl    { osreport };                 // clobbers r0,r3-r12,LR,CTR
                lwz   r4, { MNUMWRITES_OFF }(r31);  // reload (clobbered above)
                cmpwi r4, { SAVE_BUFFER_SIZE_MIN };
                ble   ok;                           // fits (== exactly full is legal)
                lis   r3, fmt_overflow@h;
                addi  r3, r3, fmt_overflow@l;
                li    r5, { SAVE_BUFFER_SIZE_MIN };
                bl    { osreport };
                lis   r3, 0xDEAD;                   // recognizable DAR sentinel: self-identifies in
                ori   r3, r3, 0x0BAD;               // the crash-screen dump as our deliberate panic
                lwz   r0, 0x0(r3);                  // deliberate DSI: SAVE BUFFER OVERFLOW panic
            ok:
                .long epilogue_orig;                // displaced `lmw r24, 0x30(r1)`
                b     { epilogue + 4 };
            })
            .encoded_bytes()
        } else {
            ppcasm!(cave_addr, {
                b     skip;
                fmt:  .asciiz b"randomprime: SAVE BUFFER OVERFLOW wrote=%08x limit=%08x\n";
            skip:
                lwz   r4, { MNUMWRITES_OFF }(r31); // bytes serialized
                cmpwi r4, { SAVE_BUFFER_SIZE_MIN };
                ble   ok;                          // fits (== exactly full is legal)
                lis   r3, fmt@h;
                addi  r3, r3, fmt@l;
                li    r5, { SAVE_BUFFER_SIZE_MIN };
                bl    { osreport };                // clobbers r0,r3-r12,LR,CTR; r31 + stack-saved LR survive
                lis   r3, 0xDEAD;                  // recognizable DAR sentinel: self-identifies in the
                ori   r3, r3, 0x0BAD;              // crash-screen register dump as our deliberate panic
                lwz   r0, 0x0(r3);                 // deliberate DSI: SAVE BUFFER OVERFLOW panic
            ok:
                .long epilogue_orig;               // displaced `lmw r24, 0x30(r1)`
                b     { epilogue + 4 };
            })
            .encoded_bytes()
        }
    })?;

    Ok(())
}

// Log every asset id that CResLoader::FindResourceForLoad fails to find. A dangling id (e.g. a
// logbook scan with no resource in any loaded pak) returns null here, which LoadResourceAsync then
// derefs and crashes; this prints the id on the not-found path so it's the last line before the
// crash. NTSC 0-00 only.
fn patch_diag_resource_miss(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    if version != Version::NtscU0_00 {
        return Ok(());
    }
    let (Some(find), Some(osreport)) = (
        symbol_addr_opt!("FindResourceForLoad__10CResLoaderFUi", version),
        symbol_addr_opt!("OSReport", version),
    ) else {
        return Ok(());
    };
    // Not-found path: `li r3, 0` falling into the epilogue. The id is in r29 (saved at prologue) and
    // survives OSReport (which clobbers only r0, r3-r12, LR, CTR).
    let li_r3_0 = find + 0xe0;
    let epilogue = find + 0xe4;

    let mut fmt_bytes = b"randomprime: resource not found id=%08x\n\0".to_vec();
    while fmt_bytes.len() % 4 != 0 {
        fmt_bytes.push(0);
    }
    let fmt_addr = emitter.emit_addressed(dol_patcher, move |_| fmt_bytes.clone())?;

    emitter.emit_and_patch(dol_patcher, li_r3_0, false, |cave_addr| {
        ppcasm!(cave_addr, {
            lis  r3, { (fmt_addr >> 16) as i32 };
            ori  r3, r3, { (fmt_addr & 0xffff) as i32 };
            mr   r4, r29;            // missing asset id
            bl   { osreport };
            li   r3, 0;              // displaced instruction (return nullptr)
            b    { epilogue };
        })
        .encoded_bytes()
    })?;

    Ok(())
}

pub fn patch_dol(
    file: &mut structs::FstEntryFile,
    #[allow(unused_variables)] spawn_room: SpawnRoomData,
    config: &PatchConfig,
) -> Result<Vec<u8>, String> {
    let version = config.version;

    if version == Version::NtscUTrilogy
        || version == Version::NtscJTrilogy
        || version == Version::PalTrilogy
    {
        return Ok(Vec::new());
    }

    let reader = match *file {
        structs::FstEntryFile::Unknown(ref reader) => reader.clone(),
        _ => panic!(),
    };
    let mut dol_patcher = DolPatcher::new(reader);
    let mut emitter = TextEmitter::new(caves_for_version(version));

    patch_meta(&mut dol_patcher, &mut emitter, version, config)?;
    patch_heap_optimization(&mut dol_patcher, &mut emitter, version, config)?;

    // Reserve must-fit cave data early (emit_addressed has no overflow path); trampolines emit last.
    let save_uuid_data = reserve_save_uuid_data(&mut dol_patcher, &mut emitter, version, config)?;

    patch_os_diagnostics(&mut dol_patcher, &mut emitter, version, config)?;

    patch_game_options(&mut dol_patcher, version, config)?;
    patch_cosmetic(&mut dol_patcher, &mut emitter, version, config)?;
    patch_game_start(&mut dol_patcher, &mut emitter, version, config, spawn_room)?;
    patch_restore_ntsc_00(&mut dol_patcher, &mut emitter, version, config)?;
    patch_gameplay_tweaks(&mut dol_patcher, &mut emitter, version, config)?;

    patch_spring_ball(&mut dol_patcher, &mut emitter, version, config)?;
    patch_custom_items(&mut dol_patcher, &mut emitter, version)?;
    if config.warp_to_start {
        patch_warp_to_start(&mut dol_patcher, &mut emitter, version)?;
    }

    // Overflow-safe trampolines; emitted after the must-fit emit_addressed reservations.
    patch_inventory_gates(&mut dol_patcher, &mut emitter, version)?;

    // Emitted last for readability; these emit_and_patch stubs are overflow-safe (see make_pic).
    patch_save_uuid_stamp(&mut dol_patcher, &mut emitter, version, &save_uuid_data)?;
    patch_save_uuid_block(&mut dol_patcher, &mut emitter, version, &save_uuid_data)?;
    patch_save_name(&mut dol_patcher, &mut emitter, version, &save_uuid_data)?;
    // Unconditional: protects save integrity. Only the verbose report is gated on os_diagnostics.
    patch_save_overflow_guard(
        &mut dol_patcher,
        &mut emitter,
        version,
        config.os_diagnostics,
    )?;

    let overflow_bytes = emitter.serialize_overflow();
    *file = structs::FstEntryFile::ExternalFile(Box::new(dol_patcher));
    Ok(overflow_bytes)
}
