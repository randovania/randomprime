use std::borrow::Cow;

use dol_symbol_table::mp1_symbol;
use ppcasm::ppcasm;

use crate::custom_assets::custom_asset_ids;
use crate::dol_patcher::DolPatcher;
use crate::elevators::SpawnRoomData;
use crate::patch_config::{
    DifficultyBehavior, PatchConfig, PhazonDamageModifier, SuitDamageReduction, Version, Visor,
};
use crate::pickup_meta::PickupType;
use crate::txtr_conversions::{huerotate_color, huerotate_matrix};

macro_rules! symbol_addr {
    ($sym:tt, $version:expr) => {{
        let s = mp1_symbol!($sym);
        match &$version {
            Version::NtscU0_00 => s.addr_0_00,
            Version::NtscU0_01 => s.addr_0_01,
            Version::NtscU0_02 => s.addr_0_02,
            Version::NtscK => s.addr_kor,
            Version::NtscJ => s.addr_jpn,
            Version::Pal => s.addr_pal,
            Version::NtscUTrilogy => unreachable!(),
            Version::NtscJTrilogy => unreachable!(),
            Version::PalTrilogy => unreachable!(),
        }
        .unwrap_or_else(|| panic!("Symbol {} unknown for version {}", $sym, $version))
    }};
}

pub struct CodeCave {
    start: u32,
    size: u32,
    used: u32,
}

impl CodeCave {
    pub fn new(start: u32, size: u32) -> Self {
        CodeCave {
            start,
            size,
            used: 0,
        }
    }
    pub fn remaining(&self) -> u32 {
        self.size - self.used
    }
    fn alloc(&mut self, bytes: u32) -> u32 {
        let addr = self.start + self.used;
        self.used += bytes;
        addr
    }
}

pub struct CodeCaveAllocator {
    caves: Vec<CodeCave>,
}

impl CodeCaveAllocator {
    pub fn new(caves: Vec<CodeCave>) -> Self {
        CodeCaveAllocator { caves }
    }

    // Pick the smallest cave that still has room
    pub fn alloc(&mut self, label: &str, bytes: u32) -> u32 {
        let idx = self
            .caves
            .iter()
            .enumerate()
            .filter(|(_, c)| c.remaining() >= bytes)
            .min_by_key(|(_, c)| c.remaining())
            .map(|(i, _)| i)
            .unwrap_or_else(|| {
                panic!(
                    "CodeCaveAllocator: no cave fits {} bytes for block '{}'",
                    bytes, label
                )
            });
        self.caves[idx].alloc(bytes)
    }

    // Allocate from the start of the cave at `cave_start`. This is used for payloads
    // that are pre-linked to a fixed base address (for example rel_loader).
    pub fn alloc_from_cave_start(&mut self, label: &str, cave_start: u32, bytes: u32) -> u32 {
        let cave = self
            .caves
            .iter_mut()
            .find(|c| c.start == cave_start)
            .unwrap_or_else(|| {
                panic!(
                    "CodeCaveAllocator: no cave starts at 0x{:08x} for block '{}'",
                    cave_start, label
                )
            });
        assert!(
            cave.used == 0,
            "Code cave 0x{:08x} already consumed {} bytes before '{}'",
            cave_start,
            cave.used,
            label
        );
        assert!(
            cave.remaining() >= bytes,
            "Code cave 0x{:08x} has only {} bytes remaining for '{}'",
            cave_start,
            cave.remaining(),
            label
        );
        cave.alloc(bytes)
    }
}

pub struct TextEmitter {
    pub cave_alloc: CodeCaveAllocator,
}

impl TextEmitter {
    pub fn new(cave_alloc: CodeCaveAllocator) -> Self {
        TextEmitter { cave_alloc }
    }

    pub fn emit_at_cave_start(
        &mut self,
        dol_patcher: &mut DolPatcher<'_>,
        label: &str,
        cave_start: u32,
        bytes: Vec<u8>,
    ) -> Result<u32, String> {
        let addr = self
            .cave_alloc
            .alloc_from_cave_start(label, cave_start, bytes.len() as u32);
        dol_patcher.patch(addr, Cow::Owned(bytes))?;
        Ok(addr)
    }

    pub fn emit_addressed<F>(
        &mut self,
        dol_patcher: &mut DolPatcher<'_>,
        label: &str,
        build: F,
    ) -> Result<u32, String>
    where
        F: Fn(u32) -> Vec<u8>,
    {
        // 0x80000000 keeps all game-symbol bl targets within opcode limits
        const PROBE_ADDR: u32 = 0x80000000;
        let probe = build(PROBE_ADDR);
        let len = probe.len() as u32;
        let addr = self.cave_alloc.alloc(label, len);
        let final_bytes = build(addr);
        debug_assert_eq!(
            final_bytes.len(),
            probe.len(),
            "{} size changed under final address",
            label
        );
        dol_patcher.patch(addr, Cow::Owned(final_bytes))?;
        Ok(addr)
    }
}

pub fn caves_for_version(version: Version) -> CodeCaveAllocator {
    if version == Version::NtscUTrilogy
        || version == Version::NtscJTrilogy
        || version == Version::PalTrilogy
    {
        todo!();
    }

    let caves = vec![
        CodeCave::new(
            symbol_addr!(
                "GetDescriptionForCommand__13ControlMapperFQ213ControlMapper9ECommands",
                version
            ),
            988,
        ),
        CodeCave::new(symbol_addr!("sndStreamAllocStereo", version), 704),
        CodeCave::new(symbol_addr!("__OSModuleInit", version) + 0x18, 516),
        CodeCave::new(
            symbol_addr!(
                "GetDescriptionForFunction__13ControlMapperFQ213ControlMapper13EFunctionList",
                version
            ),
            416,
        ),
        CodeCave::new(symbol_addr!("C_MTXLookAt", version), 396),
        CodeCave::new(symbol_addr!("GXSetTexCoordBias", version), 124),
    ];
    CodeCaveAllocator::new(caves)
}

struct RelLoaderSelection {
    cave_bytes: &'static [u8],
    cave_map_str: &'static str,
    cave_start: u32,
}

fn rel_loader_selection(version: Version) -> RelLoaderSelection {
    match version {
        Version::NtscU0_00 => RelLoaderSelection {
            cave_bytes: rel_files::REL_LOADER_100_CAVE,
            cave_map_str: rel_files::REL_LOADER_100_CAVE_MAP,
            cave_start: rel_files::REL_LOADER_100_CAVE_BASE,
        },
        Version::NtscU0_01 => RelLoaderSelection {
            cave_bytes: rel_files::REL_LOADER_101_CAVE,
            cave_map_str: rel_files::REL_LOADER_101_CAVE_MAP,
            cave_start: rel_files::REL_LOADER_101_CAVE_BASE,
        },
        Version::NtscU0_02 => RelLoaderSelection {
            cave_bytes: rel_files::REL_LOADER_102_CAVE,
            cave_map_str: rel_files::REL_LOADER_102_CAVE_MAP,
            cave_start: rel_files::REL_LOADER_102_CAVE_BASE,
        },
        Version::NtscK => RelLoaderSelection {
            cave_bytes: rel_files::REL_LOADER_KOR_CAVE,
            cave_map_str: rel_files::REL_LOADER_KOR_CAVE_MAP,
            cave_start: rel_files::REL_LOADER_KOR_CAVE_BASE,
        },
        Version::NtscJ => RelLoaderSelection {
            cave_bytes: rel_files::REL_LOADER_JPN_CAVE,
            cave_map_str: rel_files::REL_LOADER_JPN_CAVE_MAP,
            cave_start: rel_files::REL_LOADER_JPN_CAVE_BASE,
        },
        Version::Pal => RelLoaderSelection {
            cave_bytes: rel_files::REL_LOADER_PAL_CAVE,
            cave_map_str: rel_files::REL_LOADER_PAL_CAVE_MAP,
            cave_start: rel_files::REL_LOADER_PAL_CAVE_BASE,
        },
        Version::NtscUTrilogy | Version::NtscJTrilogy | Version::PalTrilogy => unreachable!(),
    }
}

pub fn patch_is_memory_relay_active_func(
    addr: u32,
    g_game_state: u32,
    state_for_world: u32,
) -> Vec<u8> {
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
}

pub fn patch_set_pickup_icon_txtr_ntsc(
    addr: u32,
    is_memory_relay_active_func: u32,
    off: i32,
    map_pickup_icon_txtr: u32,
    draw_func_298: u32,
) -> Vec<u8> {
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
        lwz          r3, {off}(r13);
        lis          r6, {map_pickup_icon_txtr}@h;
        addi         r6, r6, {map_pickup_icon_txtr}@l;
        beq          {addr + 0x3c};
        fmr          f30, f14;
        b            {draw_func_298};
    })
    .encoded_bytes()
}

pub fn patch_set_pickup_icon_txtr_pal_j(
    addr: u32,
    is_memory_relay_active_func: u32,
    off: i32,
    map_pickup_icon_txtr: u32,
    draw_func_284: u32,
) -> Vec<u8> {
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
        lwz          r3, {off}(r13);
        beq          {addr + 0x4c};
        fmr          f30, f14;
        b            {draw_func_284};
    })
    .encoded_bytes()
}

pub fn patch_spring_ball_start(
    addr: u32,
    morph_ball_offset: u32,
    movement_state_offset: u32,
    out_of_water_ticks_offset: u32,
    surface_restraint_type_offset: u32,
    is_movement_allowed: u32,
) -> Vec<u8> {
    let data_addr = addr + 0x1b4;
    ppcasm!(addr, {
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
        bgt       {addr + 0x14c};
        lwz       r0, {movement_state_offset}(r14);
        cmplwi    r0, 0;
        beq       {addr + 0x84};
        b         {addr + 0x14c};
        cmplwi    r0, 4;
        bne       {addr + 0x14c};
        lwz       r0, {out_of_water_ticks_offset}(r14);
        cmplwi    r0, 2;
        bne       {addr + 0x90};
        lwz       r0, {surface_restraint_type_offset}(r14);
        b         {addr + 0x94};
        li        r0, 4;
        cmplwi    r0, 7;
        beq       {addr + 0x14c};
        mr        r3, r28;
        bl        {is_movement_allowed};
        cmplwi    r3, 0;
        beq       {addr + 0x14c};
    })
    .encoded_bytes()
}

pub fn patch_spring_ball_item_condition(
    addr: u32,
    spring_ball_start: u32,
    spring_ball_item_kind: Option<u32>,
    has_power_up: u32,
) -> Vec<u8> {
    if let Some(kind) = spring_ball_item_kind {
        ppcasm!(addr, {
            lwz       r3, 0x0(r15);
            li        r4, {kind};
            bl        {has_power_up};
            cmplwi    r3, 0;
            beq       {spring_ball_start + 0x14c};
        })
        .encoded_bytes()
    } else {
        ppcasm!(addr, {
            nop;
            nop;
            nop;
            nop;
            nop;
        })
        .encoded_bytes()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn patch_spring_ball_end(
    addr: u32,
    spring_ball_start: u32,
    attached_actor_offset: u32,
    energy_drain_offset: u32,
    velocity_offset: u32,
    get_energy_drain_intensity: u32,
    bomb_jump: u32,
    set_move_state: u32,
    compute_boost_ball: u32,
) -> Vec<u8> {
    ppcasm!(addr, {
        lhz       r0, {attached_actor_offset}(r14);
        cmplwi    r0, 65535;
        bne       {spring_ball_start + 0x14c};
        addi      r3, r14, {energy_drain_offset};
        bl        {get_energy_drain_intensity};
        fcmpu     cr0, f1, f14;
        bgt       {spring_ball_start + 0x14c};
        lwz       r0, 0x187c(r28);
        cmplwi    r0, 0;
        bne       {spring_ball_start + 0x14c};
        lfs       f1, 0x14(r29);
        fcmpu     cr0, f1, f14;
        ble       {spring_ball_start + 0x14c};
        lfs       f16, {velocity_offset}(r14);
        lfs       f17, {velocity_offset + 4}(r14);
        mr        r3, r14;
        mr        r4, r16;
        mr        r5, r30;
        bl        {bomb_jump};
        stfs      f16, {velocity_offset}(r14);
        stfs      f17, {velocity_offset + 4}(r14);
        lfs       f17, 0x1dfc(r17);
        fcmpu     cr0, f17, f14;
        ble       {spring_ball_start + 0x130};
        lfs       f17, 0x10(r16);
        lfs       f16, {velocity_offset + 8}(r14);
        fdivs     f16, f16, f17;
        stfs      f16, {velocity_offset + 8}(r14);
        mr        r3, r14;
        li        r4, 4;
        mr        r5, r29;
        bl        {set_move_state};
        li        r3, 40;
        stw       r3, 0x0c(r16);
        b         {spring_ball_start + 0x160};
        lwz       r3, 0x0c(r16);
        cmplwi    r3, 0;
        beq       {spring_ball_start + 0x160};
        addi      r3, r3, -1;
        stw       r3, 0x0c(r16);
        mr        r3, r28;
        mr        r4, r29;
        mr        r5, r30;
        fmr       f1, f15;
        bl        {compute_boost_ball};
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
    .encoded_bytes()
}

pub fn patch_spring_ball_cooldown_unmorph(
    addr: u32,
    spring_ball_cooldown: u32,
    leave_morph_ball: u32,
) -> Vec<u8> {
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
        bl        {leave_morph_ball};
        andi      r14, r14, 0;
        lwz       r0, 0x18(r1);
        lwz       r31, 0x14(r1);
        lwz       r30, 0x10(r1);
        mtlr      r0;
        addi      r1, r1, 0x18;
        blr;
    })
    .encoded_bytes()
}

pub fn patch_spring_ball_cooldown_morph(
    addr: u32,
    spring_ball_cooldown: u32,
    enter_morph_ball: u32,
) -> Vec<u8> {
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
        bl        {enter_morph_ball};
        andi      r14, r14, 0;
        lwz       r0, 0x18(r1);
        lwz       r31, 0x14(r1);
        lwz       r30, 0x10(r1);
        mtlr      r0;
        addi      r1, r1, 0x18;
        blr;
    })
    .encoded_bytes()
}

#[allow(clippy::too_many_arguments)]
pub fn patch_custom_item_initialize_power_up(
    addr: u32,
    power_up_max_values: u32,
    life_time_offset: u32,
    probability_offset: u32,
    actor_flags_offset: u32,
    out_of_water_ticks_offset: u32,
    fluid_depth_offset: u32,
    freeze: u32,
    init_power_up: u32,
    first_custom_item_idx: i32,
) -> Vec<u8> {
    ppcasm!(addr, {
        mr           r29, r4;
        mr           r14, r5;
        lis          r15, {power_up_max_values}@h;
        addi         r15, r15, {power_up_max_values}@l;
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
        bl           {freeze};
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
        b            {init_power_up + 0x108};
    continue_init_power_up:
        mr           r5, r14;
        andi         r14, r14, 0;
        andi         r15, r15, 0;
        andi         r16, r16, 0;
        fmr          f14, f28;
        fmr          f15, f28;
        cmpwi        r29, 0;
        b            {init_power_up + 0x20};
    data:
        .float    75.0;
    })
    .encoded_bytes()
}

pub fn patch_custom_item_has_power_up(
    addr: u32,
    first_custom_item_idx: i32,
    has_power_up: u32,
) -> Vec<u8> {
    ppcasm!(addr, {
        mr           r0, r3;
        lis          r3, r3_backup@h;
        addi         r3, r3, r3_backup@l;
        stw          r0, 0x0(r3);
        lwz          r3, 0x0(r3);
        mr           r0, r4;
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        stw          r0, 0x0(r4);
        lwz          r4, 0x0(r4);
        cmpwi        r4, {PickupType::ArtifactOfNewborn.kind()};
        ble          not_custom_item;
        li           r4, {PickupType::UnknownItem2.kind()};
        rlwinm       r0, r4, 0x3, 0x0, 0x1c;
        add          r4, r3, r0;
        addi         r4, r4, 0x2c;
        lwz          r0, 0x0(r4);
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        lwz          r4, 0x0(r4);
        li           r3, {first_custom_item_idx};
        add          r3, r3, r4;
        srw          r0, r3, r3;
        andi         r3, r3, 1;
    powerup_not_valid:
        blr;
    not_custom_item:
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        lwz          r4, 0x0(r4);
        cmpwi        r4, 0;
        blt          powerup_not_valid;
        b            {has_power_up + 0x8};
    r3_backup:
        .long 0;
    r4_backup:
        .long 0;
    })
    .encoded_bytes()
}

pub fn patch_custom_item_get_item_amount(addr: u32, get_item_amount: u32) -> Vec<u8> {
    ppcasm!(addr, {
        mr           r0, r3;
        lis          r3, r3_backup@h;
        addi         r3, r3, r3_backup@l;
        stw          r0, 0x0(r3);
        lwz          r3, 0x0(r3);
        mr           r0, r4;
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        stw          r0, 0x0(r4);
        lwz          r4, 0x0(r4);
        mr           r4, r3;
        li           r3, {PickupType::UnknownItem2.kind()};
        rlwinm       r3, r3, 0x3, 0x0, 0x1c;
        add          r3, r4, r3;
        addi         r3, r3, 0x2c;
        lwz          r3, 0x0(r3);
        mr           r0, r3;
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        lwz          r4, 0x0(r4);
        cmpwi        r4, {PickupType::Missile.kind()};
        bne          check_power_bomb;
        andi         r0, r3, {PickupType::MissileLauncher.custom_item_value()};
        cmpwi        r3, 0;
        beq          no_launcher;
        lis          r4, r3_backup@h;
        addi         r4, r4, r3_backup@l;
        lwz          r4, 0x0(r4);
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
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        lwz          r4, 0x0(r4);
        cmpwi        r4, {PickupType::PowerBomb.kind()};
        bne          not_unlimited_or_not_pb_missiles;
        andi         r0, r3, {PickupType::PowerBombLauncher.custom_item_value()};
        cmpwi        r3, 0;
        beq          no_launcher;
        lis          r4, r3_backup@h;
        addi         r4, r4, r3_backup@l;
        lwz          r4, 0x0(r4);
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
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        lwz          r4, 0x0(r4);
        blr;
    not_unlimited_or_not_pb_missiles:
        lis          r3, r3_backup@h;
        addi         r3, r3, r3_backup@l;
        lwz          r3, 0x0(r3);
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        lwz          r4, 0x0(r4);
        cmpwi        r4, 0;
        blt          item_type_negative;
        b            {get_item_amount + 0x8};
    item_type_negative:
        li           r3, 0;
        blr;
    r3_backup:
        .long 0;
    r4_backup:
        .long 0;
    })
    .encoded_bytes()
}

pub fn patch_custom_item_get_item_capacity(addr: u32, get_item_capacity: u32) -> Vec<u8> {
    ppcasm!(addr, {
        mr           r0, r3;
        lis          r3, r3_backup@h;
        addi         r3, r3, r3_backup@l;
        stw          r0, 0x0(r3);
        mr           r0, r4;
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        stw          r0, 0x0(r4);
        lwz          r3, 0x0(r3);
        li           r4, {PickupType::UnknownItem2.kind()};
        rlwinm       r0, r4, 0x3, 0x0, 0x1c;
        add          r4, r3, r0;
        addi         r4, r4, 0x2c;
        lwz          r0, 0x0(r4);
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        lwz          r4, 0x0(r4);
        cmpwi        r4, {PickupType::Missile.kind()};
        bne          check_power_bomb;
        andi         r0, r3, {PickupType::MissileLauncher.custom_item_value()};
        cmpwi        r3, 0;
        beq          no_launcher;
        lis          r3, r3_backup@h;
        addi         r3, r3, r3_backup@l;
        lwz          r3, 0x0(r3);
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
        lis          r3, r3_backup@h;
        addi         r3, r3, r3_backup@l;
        lwz          r3, 0x0(r3);
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
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        lwz          r4, 0x0(r4);
    powerup_not_valid:
        blr;
    not_unlimited_or_not_pb_missiles:
        lis          r3, r3_backup@h;
        addi         r3, r3, r3_backup@l;
        lwz          r3, 0x0(r3);
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        lwz          r4, 0x0(r4);
        cmpwi        r4, 0;
        blt          powerup_not_valid;
        b            {get_item_capacity + 0x8};
    r3_backup:
        .long 0;
    r4_backup:
        .long 0;
    })
    .encoded_bytes()
}

pub fn patch_custom_item_decr_pickup(addr: u32, decr_pickup: u32) -> Vec<u8> {
    ppcasm!(addr, {
        mr           r0, r3;
        lis          r3, r3_backup@h;
        addi         r3, r3, r3_backup@l;
        stw          r0, 0x0(r3);
        mr           r0, r4;
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        stw          r0, 0x0(r4);
        lwz          r3, 0x0(r3);
        li           r4, {PickupType::UnknownItem2.kind()};
        rlwinm       r0, r4, 0x3, 0x0, 0x1c;
        add          r4, r3, r0;
        addi         r4, r4, 0x28;
        lwz          r0, 0x0(r4);
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        lwz          r4, 0x0(r4);
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
        lis          r3, r3_backup@h;
        addi         r3, r3, r3_backup@l;
        lwz          r3, 0x0(r3);
        lis          r4, r4_backup@h;
        addi         r4, r4, r4_backup@l;
        lwz          r4, 0x0(r4);
        cmpwi        r0, 0;
        beq          not_unlimited;
    powerup_not_valid:
        blr;
    not_unlimited:
        cmpwi        r4, 0;
        blt          powerup_not_valid;
        b            {decr_pickup + 0x8};
    r3_backup:
        .long 0;
    r4_backup:
        .long 0;
    })
    .encoded_bytes()
}

pub fn patch_restore_original_check_code_cave(addr: u32, cridley_acceptscriptmsg: u32) -> Vec<u8> {
    ppcasm!(addr, {
        lbz       r0, 0x0140(r3);
        rlwinm.   r0, r0, 26, 31, 31;
        bne       {addr + 0x18};
        lwz       r0, 0x13c(r3);
        cmpwi     r0, 6;
        bne       {addr + 0x20};
        fmr       f0, f14;
        stfs      f0, 0xad0(r30);
        b         {cridley_acceptscriptmsg + 0x898};
    })
    .encoded_bytes()
}

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
    emitter.emit_at_cave_start(
        dol_patcher,
        "rel_loader",
        rel_loader.cave_start,
        rel_loader_bytes,
    )?;
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
    emitter.emit_addressed(dol_patcher, "is_memory_relay_active_func", |addr| {
        patch_is_memory_relay_active_func(
            addr,
            symbol_addr!("g_GameState", version),
            symbol_addr!("StateForWorld__10CGameStateFUi", version),
        )
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
        emitter.emit_addressed(dol_patcher, "sitp_pal_j", |addr| {
            patch_set_pickup_icon_txtr_pal_j(
                addr,
                is_memory_relay_active_func,
                sitp_off,
                map_pickup_icon_txtr,
                draw_func + 0x284,
            )
        })?
    } else {
        emitter.emit_addressed(dol_patcher, "sitp_ntsc", |addr| {
            patch_set_pickup_icon_txtr_ntsc(
                addr,
                is_memory_relay_active_func,
                sitp_off,
                map_pickup_icon_txtr,
                draw_func + 0x298,
            )
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
    let wts_addr = emitter.emit_addressed(dol_patcher, "warp_to_start", |addr| {
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
    dol_patcher.ppcasm_patch(&ppcasm!(think_save_station + 0x54, {
        b { wts_addr };
    }))?;
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

    // All three spring-ball parts are one contiguous allocation (conditional branches
    // between them use the +-32 KB BC form and require contiguity).
    let compute_spring_ball_movement =
        emitter.emit_addressed(dol_patcher, "spring_ball", |base| {
            let sb_start = patch_spring_ball_start(
                base,
                morph_ball_offset,
                movement_state_offset,
                sb_out_of_water_ticks_offset,
                surface_restraint_type_offset,
                is_movement_allowed_sym,
            );
            let sb_item_addr = base + sb_start.len() as u32;
            let sb_item = patch_spring_ball_item_condition(
                sb_item_addr,
                base,
                spring_ball_item_kind,
                has_power_up_sym,
            );
            let sb_end_addr = sb_item_addr + sb_item.len() as u32;
            let sb_end = patch_spring_ball_end(
                sb_end_addr,
                base,
                attached_actor_offset,
                energy_drain_offset,
                velocity_offset,
                get_energy_drain_sym,
                bomb_jump_sym,
                set_move_state_sym,
                compute_boost_ball_sym,
            );
            let mut all = sb_start;
            all.extend(sb_item);
            all.extend(sb_end);
            all
        })?;
    #[rustfmt::skip]
    dol_patcher.ppcasm_patch(&ppcasm!(
        symbol_addr!("ComputeBallMovement__10CMorphBallFRC11CFinalInputR13CStateManagerf", version) + 0x2c,
        { bl {compute_spring_ball_movement}; }))?;

    // spring_ball_cooldown is the .long 0 word embedded in patch_spring_ball_end's data section.
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

    let sb_unmorph_addr = emitter.emit_addressed(dol_patcher, "sb_unmorph", |addr| {
        patch_spring_ball_cooldown_unmorph(addr, spring_ball_cooldown, leave_morph_ball_sym)
    })?;
    dol_patcher.ppcasm_patch(&ppcasm!(
        update_morph_ball_transition + call_leave_morph_ball_offset,
        {
            bl { sb_unmorph_addr };
        }
    ))?;

    let sb_morph_addr = emitter.emit_addressed(dol_patcher, "sb_morph", |addr| {
        patch_spring_ball_cooldown_morph(addr, spring_ball_cooldown, enter_morph_ball_sym)
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

    let ci_init_addr = emitter.emit_addressed(dol_patcher, "ci_init", |addr| {
        patch_custom_item_initialize_power_up(
            addr,
            power_up_max_values_sym,
            life_time_offset,
            probability_offset,
            actor_flags_offset,
            out_of_water_ticks_offset,
            fluid_depth_offset,
            freeze_sym,
            init_power_up_sym,
            first_custom_item_idx,
        )
    })?;
    dol_patcher.ppcasm_patch(&ppcasm!(init_power_up_sym + 0x1c, {
        b { ci_init_addr };
    }))?;

    let has_power_up_sym = symbol_addr!(
        "HasPowerUp__12CPlayerStateCFQ212CPlayerState9EItemType",
        version
    );
    let ci_has_addr = emitter.emit_addressed(dol_patcher, "ci_has", |addr| {
        patch_custom_item_has_power_up(addr, first_custom_item_idx, has_power_up_sym)
    })?;
    dol_patcher.ppcasm_patch(&ppcasm!(has_power_up_sym, {
        b { ci_has_addr };
        nop;
    }))?;

    let get_item_amount_sym = symbol_addr!(
        "GetItemAmount__12CPlayerStateCFQ212CPlayerState9EItemType",
        version
    );
    let ci_amount_addr = emitter.emit_addressed(dol_patcher, "ci_amount", |addr| {
        patch_custom_item_get_item_amount(addr, get_item_amount_sym)
    })?;
    dol_patcher.ppcasm_patch(&ppcasm!(get_item_amount_sym, {
        b { ci_amount_addr };
        nop;
    }))?;

    let get_item_capacity_sym = symbol_addr!(
        "GetItemCapacity__12CPlayerStateCFQ212CPlayerState9EItemType",
        version
    );
    let ci_capacity_addr = emitter.emit_addressed(dol_patcher, "ci_capacity", |addr| {
        patch_custom_item_get_item_capacity(addr, get_item_capacity_sym)
    })?;
    dol_patcher.ppcasm_patch(&ppcasm!(get_item_capacity_sym, {
        b { ci_capacity_addr };
        nop;
    }))?;

    let decr_pickup_sym = symbol_addr!(
        "DecrPickUp__12CPlayerStateFQ212CPlayerState9EItemTypei",
        version
    );
    let ci_decr_addr = emitter.emit_addressed(dol_patcher, "ci_decr", |addr| {
        patch_custom_item_decr_pickup(addr, decr_pickup_sym)
    })?;
    dol_patcher.ppcasm_patch(&ppcasm!(decr_pickup_sym, {
        b { ci_decr_addr };
        nop;
    }))?;

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
        let restore_addr = emitter.emit_addressed(dol_patcher, "restore_ridley_check", |addr| {
            patch_restore_original_check_code_cave(addr, cridley_addr)
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

fn patch_meta(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    config: &PatchConfig,
) -> Result<(), String> {
    patch_rel_loader(dol_patcher, emitter, version)?;

    if let Some(uuid) = config.uuid {
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
        dol_patcher.patch(build_info_address, uuid.to_vec().into())?;
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

    if let Some(bytes) = &config.update_hint_state_replacement {
        dol_patcher.patch(
            symbol_addr!("UpdateHintState__13CStateManagerFf", version),
            Cow::from(bytes.clone()),
        )?;
    }

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
    if config.starting_visor != Visor::Combat {
        let visor = config.starting_visor as u16;
        let no_starting_visor = !config.starting_items.combat_visor
            && !config.starting_items.scan_visor
            && !config.starting_items.thermal_visor
            && !config.starting_items.xray;

        if no_starting_visor {
            let scan_visor = Visor::Scan as u16;
            dol_patcher.ppcasm_patch(
                &ppcasm!(symbol_addr!("__ct__12CPlayerStateFv", version) + 0x68, {
                    li r0, scan_visor; stw r0, 0x14(r31); stw r0, 0x18(r31);
                }),
            )?;
            dol_patcher.ppcasm_patch(
                &ppcasm!(symbol_addr!("__ct__12CPlayerStateFR12CInputStream", version) + 0x70, {
                    li r0, scan_visor; stw r0, 0x14(r30); stw r0, 0x18(r30);
                }),
            )?;
            dol_patcher.ppcasm_patch(
                &ppcasm!(symbol_addr!("ResetVisor__12CPlayerStateFv", version), {
                    li r0, scan_visor; stw r0, 0x14(r3); stw r0, 0x18(r3); nop; nop;
                }),
            )?;
        } else {
            dol_patcher.ppcasm_patch(
                &ppcasm!(symbol_addr!("__ct__12CPlayerStateFv", version) + 0x68, {
                    li r0, visor; stw r6, 0x14(r31); stw r0, 0x18(r31);
                }),
            )?;
            dol_patcher.ppcasm_patch(
                &ppcasm!(symbol_addr!("__ct__12CPlayerStateFR12CInputStream", version) + 0x70, {
                    li r0, visor; stw r5, 0x14(r30); stw r0, 0x18(r30);
                }),
            )?;
            dol_patcher.ppcasm_patch(
                &ppcasm!(symbol_addr!("ResetVisor__12CPlayerStateFv", version), {
                    li r0, 0; stw r0, 0x14(r3); li r0, visor; stw r0, 0x18(r3); nop;
                }),
            )?;
        }

        let visor_item = match config.starting_visor {
            Visor::Combat => 17,
            Visor::Scan => 5,
            Visor::Thermal => 9,
            Visor::XRay => 13,
        };

        if config.starting_visor == Visor::Scan || no_starting_visor {
            let patch_offset = if version == Version::Pal || version == Version::NtscJ {
                0x3bc
            } else {
                0x434
            };
            dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("__ct__7CPlayerF9TUniqueIdRC12CTransform4fRC6CAABoxUi9CVector3fffffRC13CMaterialList", version) + patch_offset, {
                li r0, 0;
            }))?;
            let (po, po2) = if version == Version::Pal || version == Version::NtscJ {
                (0x79c, 0x7a8)
            } else {
                (0x7c8, 0x7d4)
            };
            dol_patcher.ppcasm_patch(&ppcasm!(
                symbol_addr!(
                    "TransitionFromMorphBallState__7CPlayerFR13CStateManager",
                    version
                ) + po,
                {
                    nop;
                }
            ))?;
            dol_patcher.ppcasm_patch(&ppcasm!(
                symbol_addr!(
                    "TransitionFromMorphBallState__7CPlayerFR13CStateManager",
                    version
                ) + po2,
                {
                    nop;
                }
            ))?;
            let (po, po2) = if version == Version::Pal || version == Version::NtscJ {
                (0x14c, 0x158)
            } else {
                (0x1a4, 0x1b0)
            };
            dol_patcher.ppcasm_patch(&ppcasm!(
                symbol_addr!("LeaveMorphBallState__7CPlayerFR13CStateManager", version) + po,
                {
                    nop;
                }
            ))?;
            dol_patcher.ppcasm_patch(&ppcasm!(
                symbol_addr!("LeaveMorphBallState__7CPlayerFR13CStateManager", version) + po2,
                {
                    nop;
                }
            ))?;
            let po = if version == Version::Pal || version == Version::NtscJ {
                0xb0
            } else {
                0x108
            };
            dol_patcher.ppcasm_patch(&ppcasm!(
                symbol_addr!("EnterMorphBallState__7CPlayerFR13CStateManager", version) + po,
                {
                    nop;
                    nop;
                    nop;
                }
            ))?;
        } else {
            let (po, po2) = if version == Version::Pal || version == Version::NtscJ {
                (0xdc, 0xf0)
            } else {
                (0xe8, 0xfc)
            };
            dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("UpdateVisorState__7CPlayerFRC11CFinalInputfR13CStateManager", version) + po, {
                li r4, visor_item;
            }))?;
            dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("UpdateVisorState__7CPlayerFRC11CFinalInputfR13CStateManager", version) + po2, {
                li r4, visor;
            }))?;
            let po = if version == Version::Pal || version == Version::NtscJ {
                0xb0
            } else {
                0x108
            };
            dol_patcher.ppcasm_patch(&ppcasm!(
                symbol_addr!("EnterMorphBallState__7CPlayerFR13CStateManager", version) + po,
                {
                    nop;
                    nop;
                    nop;
                }
            ))?;
        }
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

pub fn patch_dol(
    file: &mut structs::FstEntryFile,
    spawn_room: SpawnRoomData,
    config: &PatchConfig,
) -> Result<(), String> {
    let version = config.version;

    if version == Version::NtscUTrilogy
        || version == Version::NtscJTrilogy
        || version == Version::PalTrilogy
    {
        return Ok(());
    }

    let reader = match *file {
        structs::FstEntryFile::Unknown(ref reader) => reader.clone(),
        _ => panic!(),
    };
    let mut dol_patcher = DolPatcher::new(reader);
    let mut emitter = TextEmitter::new(caves_for_version(version));

    patch_meta(&mut dol_patcher, &mut emitter, version, config)?;
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

    *file = structs::FstEntryFile::ExternalFile(Box::new(dol_patcher));
    Ok(())
}
