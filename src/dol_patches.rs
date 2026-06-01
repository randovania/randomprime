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

// Dummy base address used to compile overflow stubs before their heap address is known.
// Must be above all DOL addresses (0x80xxxxxx) so external branch detection works:
// any branch whose target < STUB_COMPILE_BASE is an external DOL reference.
const STUB_COMPILE_BASE: u32 = 0x81000000;

pub struct HeapOverflowStub {
    dol_patch_site: u32,
    patch_is_bl: bool,
    stub_bytes: Vec<u8>, // position-independent; external b/bl expanded to lis+ori+mtctr+b/bctrl
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

    // Pick the smallest cave that still has room. Returns None when no cave fits.
    pub fn alloc(&mut self, bytes: u32) -> Option<u32> {
        let idx = self
            .caves
            .iter()
            .enumerate()
            .filter(|(_, c)| c.remaining() >= bytes)
            .min_by_key(|(_, c)| c.remaining())
            .map(|(i, _)| i)?;
        Some(self.caves[idx].alloc(bytes))
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
    pub overflow_stubs: Vec<HeapOverflowStub>,
}

impl TextEmitter {
    pub fn new(cave_alloc: CodeCaveAllocator) -> Self {
        TextEmitter {
            cave_alloc,
            overflow_stubs: Vec::new(),
        }
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
        let addr = self.cave_alloc.alloc(len).unwrap_or_else(|| {
            panic!(
                "CodeCaveAllocator: no cave fits {} bytes for block '{}'",
                len, label
            )
        });
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

    // Tries to allocate from caves first. On overflow, builds a position-independent
    // stub and defers the DOL patch to the REL loader via serialize_overflow().
    // Only use for stubs that are pure code with no embedded data words.
    pub fn emit_and_patch<F>(
        &mut self,
        dol_patcher: &mut DolPatcher<'_>,
        label: &str,
        dol_patch_site: u32,
        patch_is_bl: bool,
        build: F,
    ) -> Result<(), String>
    where
        F: Fn(u32) -> Vec<u8>,
    {
        const PROBE_ADDR: u32 = 0x80000000;
        let probe = build(PROBE_ADDR);
        let len = probe.len() as u32;

        if let Some(cave_addr) = self.cave_alloc.alloc(len) {
            let final_bytes = build(cave_addr);
            debug_assert_eq!(
                final_bytes.len(),
                probe.len(),
                "{} size changed under final address",
                label
            );
            dol_patcher.patch(cave_addr, Cow::Owned(final_bytes))?;
            let rel = (cave_addr as i64 - dol_patch_site as i64) as u64;
            let lk: u32 = if patch_is_bl { 1 } else { 0 };
            let branch = (0x48000000u32 | lk) | (rel & 0x03FF_FFFC) as u32;
            dol_patcher.patch(dol_patch_site, Cow::Owned(branch.to_be_bytes().to_vec()))?;
        } else {
            let raw_bytes = build(STUB_COMPILE_BASE);
            let pic_bytes = make_pic(&raw_bytes, STUB_COMPILE_BASE);
            self.overflow_stubs.push(HeapOverflowStub {
                dol_patch_site,
                patch_is_bl,
                stub_bytes: pic_bytes,
            });
        }
        Ok(())
    }

    // Serializes overflow stubs to the cave_overflow.bin wire format.
    // Returns an empty Vec when there are no overflow stubs.
    pub fn serialize_overflow(&self) -> Vec<u8> {
        if self.overflow_stubs.is_empty() {
            return Vec::new();
        }
        let stubs_total: u32 = self
            .overflow_stubs
            .iter()
            .map(|s| s.stub_bytes.len() as u32)
            .sum();
        let mut out = Vec::new();
        out.extend_from_slice(&(self.overflow_stubs.len() as u32).to_be_bytes());
        out.extend_from_slice(&stubs_total.to_be_bytes());
        for stub in &self.overflow_stubs {
            out.extend_from_slice(&stub.dol_patch_site.to_be_bytes());
            out.extend_from_slice(&(stub.patch_is_bl as u32).to_be_bytes());
            out.extend_from_slice(&(stub.stub_bytes.len() as u32).to_be_bytes());
            out.extend_from_slice(&stub.stub_bytes);
        }
        out
    }
}

// Replaces external b/bl instructions in a stub with position-independent equivalents.
// External = branch target is below stub_base (i.e., into the DOL at 0x80xxxxxx).
// Internal branches (within the stub) are left unchanged.
//   b  target -> lis r12, target@h; ori r12, r12, target@l; mtctr r12; bctr
//   bl target -> lis r0,  target@h; ori r0,  r0,  target@l; mtctr r0;  bctrl
// Only call this for pure-code stubs without embedded data words.
fn make_pic(stub_bytes: &[u8], stub_base: u32) -> Vec<u8> {
    assert_eq!(stub_bytes.len() % 4, 0);
    let stub_size = stub_bytes.len() as u32;
    let mut out = Vec::with_capacity(stub_bytes.len() * 2);
    let mut i = 0usize;
    while i < stub_bytes.len() {
        let instr = u32::from_be_bytes(stub_bytes[i..i + 4].try_into().unwrap());
        // opcode 18 (0b010010) with AA=0: b or bl with relative addressing
        if (instr & 0xFC00_0002) == 0x4800_0000 {
            let li_raw = (instr >> 2) & 0x00FF_FFFF;
            let li: i32 = if li_raw >= 0x80_0000 {
                li_raw as i32 - 0x100_0000
            } else {
                li_raw as i32
            };
            let pc = stub_base + i as u32;
            let target = (pc as i32 + li * 4) as u32;
            if target < stub_base || target >= stub_base + stub_size {
                let lk = instr & 1;
                let hi = target >> 16;
                let lo = target & 0xFFFF;
                if lk == 0 {
                    out.extend_from_slice(&(0x3D80_0000u32 | hi).to_be_bytes()); // lis r12
                    out.extend_from_slice(&(0x618C_0000u32 | lo).to_be_bytes()); // ori r12, r12
                    out.extend_from_slice(&0x7D89_03A6u32.to_be_bytes()); // mtctr r12
                    out.extend_from_slice(&0x4E80_0420u32.to_be_bytes()); // bctr
                } else {
                    out.extend_from_slice(&(0x3C00_0000u32 | hi).to_be_bytes()); // lis r0
                    out.extend_from_slice(&(0x6000_0000u32 | lo).to_be_bytes()); // ori r0, r0
                    out.extend_from_slice(&0x7C09_03A6u32.to_be_bytes()); // mtctr r0
                    out.extend_from_slice(&0x4E80_0421u32.to_be_bytes()); // bctrl
                }
                i += 4;
                continue;
            }
        }
        out.extend_from_slice(&stub_bytes[i..i + 4]);
        i += 4;
    }
    out
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
    // r6/r7 are free volatiles (GetItemCapacity takes only r3, r4).
    // Using registers instead of inline data makes this stub PIC-safe when heap-overflowed.
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
        b            {get_item_capacity + 0x8};
    })
    .encoded_bytes()
}

pub fn patch_custom_item_decr_pickup(addr: u32, decr_pickup: u32) -> Vec<u8> {
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

    // ci_* stubs are overflow-eligible: their only data words are .long 0 (opcode 0,
    // not matched by make_pic's b/bl check) and external branches target DOL addresses.
    emitter.emit_and_patch(
        dol_patcher,
        "ci_init",
        init_power_up_sym + 0x1c,
        false,
        |addr| {
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
        },
    )?;

    let has_power_up_sym = symbol_addr!(
        "HasPowerUp__12CPlayerStateCFQ212CPlayerState9EItemType",
        version
    );
    emitter.emit_and_patch(dol_patcher, "ci_has", has_power_up_sym, false, |addr| {
        patch_custom_item_has_power_up(addr, first_custom_item_idx, has_power_up_sym)
    })?;

    let get_item_amount_sym = symbol_addr!(
        "GetItemAmount__12CPlayerStateCFQ212CPlayerState9EItemType",
        version
    );
    emitter.emit_and_patch(
        dol_patcher,
        "ci_amount",
        get_item_amount_sym,
        false,
        |addr| patch_custom_item_get_item_amount(addr, get_item_amount_sym),
    )?;

    let get_item_capacity_sym = symbol_addr!(
        "GetItemCapacity__12CPlayerStateCFQ212CPlayerState9EItemType",
        version
    );
    emitter.emit_and_patch(
        dol_patcher,
        "ci_capacity",
        get_item_capacity_sym,
        false,
        |addr| patch_custom_item_get_item_capacity(addr, get_item_capacity_sym),
    )?;

    let decr_pickup_sym = symbol_addr!(
        "DecrPickUp__12CPlayerStateFQ212CPlayerState9EItemTypei",
        version
    );
    emitter.emit_and_patch(dol_patcher, "ci_decr", decr_pickup_sym, false, |addr| {
        patch_custom_item_decr_pickup(addr, decr_pickup_sym)
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

// Make CMemory::Alloc return null on allocation failure instead of calling rs_debugger_printf,
// which unconditionally crashes via ErrorHandler/OSFatal. Without this patch, any failed
// heap allocation (e.g. during CSamusDoll construction) freezes the game before Alloc even
// returns. With this patch, Alloc returns null and the caller can handle it gracefully.
//
// Implementation: CMemory::Alloc contains a conditional branch at +0x64:
//   mr. r31, r3                    ; r31 = alloc result; set CR0
//   bne .epilogue                  ; if non-null, skip error
//   ... setup args ...
//   bl rs_debugger_printf          ; CRASH: calls ErrorHandler(0xff,...,0xd1dd0d1e)
// .epilogue:
//   lbz r3, 0x8(r1)
//   bl OSRestoreInterrupts
//   mr r3, r31                     ; return result (null if alloc failed)
//   ...
//   blr
//
// We replace the bne with an unconditional b, always jumping to the epilogue.
// OSRestoreInterrupts still executes, and null is returned to the caller.
// Works together with patch_build_async_null_guard which handles the null return.
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

    // Replace bne .epilogue with b .epilogue at Alloc+0x64.
    dol_patcher.ppcasm_patch(&ppcasm!(alloc_addr + 0x64, {
        b {alloc_addr + 0x7C};
    }))?;

    Ok(())
}

// Eliminate the 1-2 second freeze (and controller-rumble buzz) that occurs when CGameAllocator
// fails to find a free block. The freeze comes from two sources in CGameAllocator::Alloc:
//
//   1. OOM callback (x58_oomCallback): CStateManager registers SwapOutAllPossibleMemory as this
//      callback. That function calls CARAMManager::WaitForAllDMAsToComplete(), which spins on
//      ARAM DMA hardware with interrupts disabled -- approximately 1 second.
//   2. DumpAllocations: Called unconditionally after the callback path. It iterates every heap
//      block and calls CStopwatch::Wait(0.005f) every 4 blocks -- roughly 0.5 seconds total.
//
// In CGameAllocator::Alloc at +0x284 there is a branch:
//   cmplwi r12, 0x0       ; is OOM callback registered?
//   beq  skip_callback    ; if not, jump to DumpAllocations path
//   ... setup + bctrl     ; call OOM callback (SwapOutAllPossibleMemory)
//   ... retry alloc ...
//   bl DumpAllocations    ; at +0x31C; iterates blocks with 5ms waits
//   li r3, 0              ; at +0x320; return null
//
// We replace the beq with an unconditional b that jumps directly to "li r3, 0", skipping both
// the OOM callback invocation and DumpAllocations entirely. Allocation simply returns null
// immediately, which is then handled gracefully by patch_alloc_null_on_failure and
// patch_build_async_null_guard.
//
// The offset +0x284 and jump distance +0xA0 are identical across all five supported versions
// (verified by binary pattern search against all production ISOs).
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

    // Replace beq (skip to DumpAllocations path) with b (skip to li r3,0 / null return).
    dol_patcher.ppcasm_patch(&ppcasm!(alloc_addr + 0x284, {
        b {alloc_addr + 0x284 + 0xA0};
    }))?;

    Ok(())
}

// Null-guard CResFactory::BuildAsync so that a failed heap allocation does not crash the game.
// When the allocator returns null (e.g. severe fragmentation during CSamusDoll construction),
// BuildAsync returns early leaving *ppObj == null. CSamusDoll::IsLoaded() then returns false,
// and CSamusDoll::Draw() already guards on IsLoaded() and skips rendering gracefully.
// Requires patch_alloc_null_on_failure to be applied first so that Alloc actually returns null.
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

    // BuildAsync frame layout (size 0x70):
    //   0x54(r1): stmw r25 - saves r25..r31
    //   0x74(r1): saved LR (caller's return address)
    // The epilogue at +0xC8 restores all of this and returns.

    let cave_addr =
        emitter.emit_addressed(dol_patcher, "build_async_null_guard_cave", |cave_addr| {
            ppcasm!(cave_addr, {
                cmpwi r29, 0x0;                                // null alloc result?
                bne do_load;
                b {build_async_addr + 0xC8};                   // early return via BuildAsync epilogue
            do_load:
                mr r4, r26;                                    // original instr: mr r4, r26
                addi r3, r25, 0x4;                             // original instr: addi r3, r25, 0x4
                mr r5, r29;                                    // original instr: mr r5, r29
                b {load_resource_async_addr};                  // tail call; LR = return addr of bl cave
            })
            .encoded_bytes()
        })?;

    // Patch 4 instructions in BuildAsync at offset +0x6C:
    //   Original: mr r4,r26 | addi r3,r25,4 | mr r5,r29 | bl LoadResourceAsync
    //   Patched:  bl cave   | b +0x7C        | nop        | nop
    // After LoadResourceAsync returns to (bl cave)+4 = +0x70, the b +0x7C skips the now-dead
    // instructions and resumes at the mr r30,r3 that consumes the return value.
    dol_patcher.ppcasm_patch(&ppcasm!(build_async_addr + 0x6C, {
        bl {cave_addr};
        b {build_async_addr + 0x7C};
        nop;
        nop;
    }))?;

    Ok(())
}

// Intercept OOM during resource decompression so the game retries instead of crashing.
// Only applied for NTSC 0-00; other versions lack symbol table entries for the unnamed
// functions and have not been verified.
//
// fn_803394A8 inflates PAK-compressed resources. On OOM, the inflate output buffer
// (SLD_inner[0x20]) is null. The inflate loop at 0x803396A8 loads that null pointer
// and passes it as z_stream.next_out, crashing inside inflate().
//
// The stub intercepts the load. On null (OOM):
//   1. Call inflateEnd to free the ~10KB zlib internal state.
//   2. Clear SLD_inner's z_stream reference (small 56-byte struct is accepted as a leak).
//   3. Jump to fn_803394A8's failure epilogue (0x80339868), which restores registers
//      from the stack frame and returns 0 to PumpResource.
// PumpResource returns 0; AsyncIdle retries next frame. If memory later frees up,
// the retry succeeds. The 56-byte z_stream leak is bounded to one per OOM event.
fn patch_inflate_null_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    if version != Version::NtscU0_00 {
        return Ok(());
    }

    let oom_flag_addr = {
        emitter.emit_addressed(dol_patcher, "inflate_oom_flag", |_| {
            0u32.to_be_bytes().to_vec()
        })?
    };

    // Hardcoded 0-00 addresses (unnamed functions not in symbol table)
    let inflate_oom_site: u32 = 0x803396A8; // lwz r3, 0x20(r30) -- OOM intercept site
    let inflate_fail_addr: u32 = 0x80339868; // fn_803394A8 failure epilogue: li r3,0; restore; blr
    let inflate_end_addr = symbol_addr!("inflateEnd", version);

    // r30 = SLD_inner, r23 = z_stream ptr (both set up by fn_803394A8 before the loop)
    // Non-OOM (r3 != 0): return normally via blr; caller continues with valid inflate buf.
    // OOM (r3 == 0): inflateEnd frees internal zlib state, clear SLD_inner z_stream
    //   reference, set oom_flag so patch_build_retry_guard can detect the OOM cause,
    //   then jump to failure epilogue. Next retry re-allocates z_stream + buf fresh.
    // r12 is volatile (may be clobbered by inflateEnd), so reload fresh with lis before use.
    let stub_addr =
        emitter.emit_addressed(dol_patcher, "inflate_null_guard_stub", |cave_addr| {
            ppcasm!(cave_addr, {
                lwz r3, 0x20(r30);              // original instruction: load inflate buf ptr
                cmpwi r3, 0x0;
                bne no_oom;                     // non-null: return r3 normally
                mr r3, r23;                     // r3 = z_stream ptr
                bl {inflate_end_addr};          // inflateEnd(z_stream) -- frees internal state
                li r0, 0;
                stb r0, 0x24(r30);              // SLD_inner[0x24] = 0 (release z_stream ownership)
                stw r0, 0x28(r30);              // SLD_inner[0x28] = null (forget z_stream ptr)
                lis r12, {oom_flag_addr}@h;     // signal OOM to Build's retry guard
                li r3, 0x1;
                stw r3, {oom_flag_addr}@l(r12);
                b {inflate_fail_addr};          // jump to fn_803394A8 failure epilogue
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

// Null-guard the medium-pool expansion inside CGameAllocator::Alloc so that a failed inner
// allocator call does not crash the game.
//
// When CGameAllocator::Alloc cannot satisfy a request from the existing medium pool, it calls
// itself recursively (via vtable) to allocate a new 0x21000-byte expansion block, then passes
// the result to CMediumAllocPool::AddPuddle. patch_alloc_null_on_failure causes that inner call
// to return null. Without this guard, the null is forwarded to AddPuddle, which computes
// (0 + capacity*32 = 0x20000) as the bookkeeping pointer and crashes writing to address 0x20000.
//
// The fix: if the inner bctrl returns null, skip AddPuddle and fall through to the pool-retry
// path at 0x80351FAC, which attempts CMediumAllocPool::Alloc once more (still fails, r23 = 0)
// and then reaches the normal OOM-handling path at 0x80351FE0 that returns null to the caller.
//
// Only applied for NTSC 0-00; these are unnamed internal functions absent from other symbol tables.
fn patch_add_puddle_null_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    if version != Version::NtscU0_00 {
        return Ok(());
    }

    // NTSC 0-00 hardcoded addresses
    let intercept_addr: u32 = 0x80351F94; // mr r0, r3 -- first instr after bctrl in CGameAllocator::Alloc
    let add_puddle_addr: u32 = 0x80350990; // CMediumAllocPool::AddPuddle
    let retry_addr: u32 = 0x80351FAC; // CMediumAllocPool::Alloc retry after AddPuddle

    // After bctrl: r3 = inner alloc result (null on OOM), r31 = this (CGameAllocator*).
    // Non-null path: replay original instrs, tail-call AddPuddle (returns to LR=intercept+4).
    //   The nop at intercept+4 is a b {retry_addr} that continues execution normally.
    // Null path: blr back to intercept+4 (= b {retry_addr}), skip AddPuddle entirely.
    //   Pool retry fails (pool not expanded), r23 = 0, falls into OOM handling at 0x80351FE0.
    let stub_addr =
        emitter.emit_addressed(dol_patcher, "add_puddle_null_guard_stub", |cave_addr| {
            ppcasm!(cave_addr, {
                cmpwi r3, 0x0;
                beq null_path;
                mr r0, r3;             // original instruction
                lwz r3, 0x74(r31);    // original instruction
                mr r5, r0;             // original instruction
                li r4, 0x1000;         // original instruction
                li r6, 0x1;            // original instruction
                b {add_puddle_addr};   // tail call; AddPuddle blr returns to LR=intercept+4
            null_path:
                blr;                   // return to intercept+4 = b {retry_addr}
            })
            .encoded_bytes()
        })?;

    // Replace 6 instructions starting at intercept_addr:
    //   Original: mr r0,r3 | lwz r3,0x74(r31) | mr r5,r0 | li r4,0x1000 | li r6,0x1 | bl AddPuddle
    //   Patched:  bl stub  | b {retry_addr}   | nop      | nop          | nop       | nop
    // AddPuddle and the null path both return to LR=intercept+4, which is the b {retry_addr}.
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

// Null-guard CTexture::InitBitmapBuffers so that a failed bitmap buffer allocation does not crash.
// When the heap returns null (OOM during beam-switch or large texture load), skips PostConstruct
// and CountMemory and jumps to the function epilogue, leaving CARAMToken in its default-
// constructed state (state field = 6). LoadToARAM checks state==6 and returns 0 early, so no
// downstream crash occurs.
// Requires patch_alloc_null_on_failure to be applied first so that Alloc actually returns null.
//
// Only applied for NTSC 0-00; InitBitmapBuffers address is absent from other symbol tables.
fn patch_init_bitmap_buffers_null_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    if version != Version::NtscU0_00 {
        return Ok(());
    }

    // NTSC 0-00 hardcoded addresses (function not in other versions' symbol tables)
    let intercept_addr: u32 = 0x8030EAD4; // first instruction after bl Alloc in InitBitmapBuffers
    let epilogue_addr: u32 = 0x8030EAF0; // InitBitmapBuffers epilogue (lwz r0, 0x24(r1))

    // After bl Alloc returns: r3 = alloc result, r31 = this (CTexture*).
    // Non-OOM path: replay original lwz r5,0xc(r31) and return to normal flow.
    // OOM path (r3==0): zero x0c_bmpDataSize, skip PostConstruct+CountMemory, branch to epilogue.
    //   CARAMToken at this+0x44 stays default-constructed (state=6); LoadToARAM returns 0 safely.
    emitter.emit_and_patch(
        dol_patcher,
        "init_bitmap_buffers_null_guard_stub",
        intercept_addr,
        true,
        |cave_addr| {
            ppcasm!(cave_addr, {
                cmpwi r3, 0x0;
                beq oom;
                lwz r5, 0xc(r31);  // original instruction: load bmpDataSize into r5
                blr;               // return; continues with mr r4,r3 then bl PostConstruct
            oom:
                li r0, 0;
                stw r0, 0xc(r31); // zero x0c_bmpDataSize (no buffer was allocated)
                b {epilogue_addr}; // skip PostConstruct + CountMemory
            })
            .encoded_bytes()
        },
    )?;

    Ok(())
}

fn patch_morph_transition_oom_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    if version != Version::NtscU0_00 {
        return Ok(());
    }

    // Intercept CanEnterMorphBallState and CanLeaveMorphBallState instead of
    // TransitionToMorphBallState/TransitionFromMorphBallState to avoid patching
    // inside the crash chain (CAnimData->memcpy, multiple null-deref sites we
    // can't fully audit). When these "can enter?" functions return false, the
    // caller already plays SFXsam_b_malfxn_00 via its pre-existing sound path,
    // and the stfs writes to player offsets 0x574/0x578 (before the transition
    // call) never happen, so no partial morph state is left.
    //
    // Both functions start with stwu (stack frame allocation). We overwrite the
    // first instruction with a b-trampoline (not bl, so LR is unchanged): the
    // OOM path does li r3,0; blr to return false directly to the caller.
    let free_bytes_addr: u32 = 0x804BFDF4;
    let threshold: u32 = 650 * 1024;

    // CanEnterMorphBallState__7CPlayerCFR13CStateManagerf (NTSC 0-00: 0x80012EFC)
    //   First instruction: stwu r1, -0x820(r1)
    let can_enter_addr: u32 = 0x80012EFC;
    emitter.emit_and_patch(
        dol_patcher,
        "can_enter_morph_oom_guard",
        can_enter_addr,
        false,
        |cave_addr| {
            ppcasm!(cave_addr, {
                lis   r12, { free_bytes_addr }@h;
                lwz   r0,  { free_bytes_addr }@l(r12);
                lis   r12, { threshold }@h;
                cmplw r0, r12;
                bge   ok;
                li    r3, 0x0;                   // OOM: return false (caller plays malfunction SFX)
                blr;
            ok:
                stwu  r1, -0x820(r1);            // trampoline: original first instruction
                b     { can_enter_addr + 4 };
            })
            .encoded_bytes()
        },
    )?;

    // CanLeaveMorphBallState__7CPlayerCFR13CStateManagerR9CVector3f (NTSC 0-00: 0x80012A94)
    //   First instruction: stwu r1, -0x980(r1)
    let can_leave_addr: u32 = 0x80012A94;
    emitter.emit_and_patch(
        dol_patcher,
        "can_leave_morph_oom_guard",
        can_leave_addr,
        false,
        |cave_addr| {
            ppcasm!(cave_addr, {
                lis   r12, { free_bytes_addr }@h;
                lwz   r0,  { free_bytes_addr }@l(r12);
                lis   r12, { threshold }@h;
                cmplw r0, r12;
                bge   ok;
                li    r3, 0x0;                   // OOM: return false (caller plays malfunction SFX)
                blr;
            ok:
                stwu  r1, -0x980(r1);            // trampoline: original first instruction
                b     { can_leave_addr + 4 };
            })
            .encoded_bytes()
        },
    )?;

    Ok(())
}

fn patch_map_open_oom_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    if version != Version::NtscU0_00 {
        return Ok(());
    }

    // CanShowMapScreen__13CStateManagerFv (NTSC 0-00: 0x800447EC) returns bool.
    // When false, CMFGame skips DeferStateTransition(kSMT_MapScreen).
    // Unlike the morph case, the game has no pre-existing sound path when this
    // returns false, so the stub plays SFXsam_b_malfxn_00 explicitly.
    //
    // b-trampoline (LR preserved in register). OOM path: push a 32-byte
    // mini-frame (16-byte linkage area + 16-byte CSfxHandle output buffer),
    // call SfxStart, restore caller LR from the mini-frame, pop, return false.
    //
    // First instruction of CanShowMapScreen: stwu r1, -0x10(r1)
    let free_bytes_addr: u32 = 0x804BFDF4;
    let threshold: u32 = 650 * 1024;
    let func_addr: u32 = 0x800447EC;
    // SfxStart__11CSfxManagerFUsssbsbi: r3=CSfxHandle* out, r4=u16 id, r5=s16 vol,
    // r6=s16 pan, r7=bool useAcoustics, r8=s16 priority, r9=bool looped, r10=s32 areaId.
    let sfx_start_addr: u32 = 0x802E9D74;

    emitter.emit_and_patch(
        dol_patcher,
        "map_open_oom_guard",
        func_addr,
        false,
        |cave_addr| {
            ppcasm!(cave_addr, {
                lis   r12, { free_bytes_addr }@h;
                lwz   r0,  { free_bytes_addr }@l(r12);
                lis   r12, { threshold }@h;
                cmplw r0, r12;
                bge   ok;
                // OOM path: push mini-frame to hold CSfxHandle output and save caller LR.
                // b-trampoline leaves caller's return address in the LR register; bl SfxStart
                // would clobber it, so we save it in the mini-frame before the call.
                stwu  r1, -0x20(r1);
                mflr  r0;
                stw   r0, 0x24(r1);              // save caller LR
                addi  r3, r1, 0x10;              // r3 = CSfxHandle output buffer
                li    r4, 0x6f5;                 // SFXsam_b_malfxn_00
                li    r5, 0x7f;                  // vol 127
                li    r6, 0x40;                  // pan center
                li    r7, 0x1;                   // useAcoustics = true
                li    r8, 0x40;                  // priority (medium)
                li    r9, 0x0;                   // not looped
                li    r10, -1;                   // areaId = -1 (all areas)
                bl    { sfx_start_addr };
                lwz   r0, 0x24(r1);              // restore caller LR
                mtlr  r0;
                addi  r1, r1, 0x20;              // pop mini-frame
                li    r3, 0x0;                   // return false (block map open)
                blr;
            ok:
                stwu  r1, -0x10(r1);             // trampoline: original first instruction
                b     { func_addr + 4 };
            })
            .encoded_bytes()
        },
    )?;

    Ok(())
}

fn patch_change_weapon_oom_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    if version != Version::NtscU0_00 {
        return Ok(());
    }

    // ChangeWeapon__10CPlayerGunFRC12CPlayerStateR13CStateManager (0x8003EF98):
    //   0x8003F00C: cmplw r3, r0   (r3=loadingBeam, r0=currentBeam)
    //   0x8003F010: beq .L_8003F03C  (skip Load if same beam)
    //   0x8003F014-0x8003F038: loadingBeam->Load + auxWeapon->Load
    //   0x8003F03C: .L_8003F03C (EnableFx / ResetBeamParams / StartWipe)
    //   0x8003F08C: function epilogue (lwz r0,0x24(r1); restore r29-r31; mtlr; addi r1; blr)
    //
    // Replace the beq with bl to a stub that:
    //   1. Honours the original beq (same beam -> 0x8003F03C for wipe animation)
    //   2. Checks heap free bytes; if sufficient, falls through to Load block
    //   3. If OOM: jumps to function EPILOGUE (0x8003F08C), skipping wipe animation
    //      entirely so the morphing flag is never set and the player can still fire.
    //      Jumping to the epilogue is safe: ChangeWeapon's prologue frame is intact.
    //
    // CR0[EQ] is preserved by bl, r0/r12 are safe to clobber (both overwritten at 0x8003F014+).
    let intercept_addr: u32 = 0x8003F010;
    // Same-beam path: honour original beq, land at EnableFx/ResetBeamParams/StartWipe.
    let skip_target: u32 = 0x8003F03C;
    // OOM path: jump past StartWipe directly to the function epilogue.
    // This prevents ResetBeamParams from setting the morphing flag, so the player
    // keeps the old beam and can fire without the morph state machine stalling.
    let epilogue_addr: u32 = 0x8003F08C;
    // Heap free-bytes counter (user-provided address).
    let free_bytes_addr: u32 = 0x804BFDF4;
    let threshold: u32 = 650 * 1024;
                                     // SfxStart__11CSfxManagerFUsssbsbi: static, r3=CSfxHandle* out, r4=id, r5=vol,
                                     // r6=pan, r7=useAcoustics, r8=priority, r9=looped, r10=areaId.
    let sfx_start_addr: u32 = 0x802E9D74;

    let stub_addr =
        emitter.emit_addressed(dol_patcher, "change_weapon_oom_guard_stub", |cave_addr| {
            ppcasm!(cave_addr, {
                bne  no_orig_skip;                        // beams differ: check OOM
                b    { skip_target };                     // beams same: honour original beq
            no_orig_skip:
                lis  r12, { free_bytes_addr }@h;
                lwz  r0,  { free_bytes_addr }@l(r12);    // r0 = heap free bytes
                lis  r12, { threshold }@h;                // r12 = threshold
                cmplw r0, r12;                            // unsigned compare
                bge  no_oom;                              // enough memory: allow Load
                // OOM path: HandleBeamChange (from ProcessInput) set x2f8_stateFlags |= 0x8
                // (morphing). ProcessInput exits early when x2f8 >= 4, so x2f4 is never
                // updated from the fire button -> player cannot fire. SetupBeam processing
                // would normally clear the morphing flag, but it only runs after a successful
                // morph, which never happens when we cancel early.
                // Restore x2f8 = 1 (beam mode) so ProcessInput reads fire input normally.
                li   r0, 0x1;
                stw  r0, 0x2f8(r29);                     // x2f8_stateFlags = 1 (beam mode)
                // Play malfunction SFX to signal blocked beam switch.
                // Push a mini-frame for the CSfxHandle output buffer (SfxStart writes to r3).
                // The epilogue at epilogue_addr uses the stack-saved LR from ChangeWeapon's
                // frame (0x24(r1)), not the LR register, so bl SfxStart clobbering LR is safe.
                // Pop the mini-frame before the epilogue jump so ChangeWeapon's frame is on top.
                stwu r1, -0x20(r1);                      // push 32-byte mini-frame
                addi r3, r1, 0x10;                       // r3 = CSfxHandle output buffer
                li   r4, 0x6f5;                          // SFXsam_b_malfxn_00
                li   r5, 0x7f;                           // vol 127
                li   r6, 0x40;                           // pan center
                li   r7, 0x1;                            // useAcoustics = true
                li   r8, 0x40;                           // priority (medium)
                li   r9, 0x0;                            // not looped
                li   r10, -1;                            // areaId = -1 (all areas)
                bl   { sfx_start_addr };
                addi r1, r1, 0x20;                       // pop mini-frame
                b    { epilogue_addr };                   // cancel beam switch, return to caller
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

fn patch_beam_load_retry(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    if version != Version::NtscU0_00 {
        return Ok(());
    }

    // ProcessGunMorph__10CPlayerGunFfR13CStateManager (0x8003AE90) polls IsLoaded()
    // on x734(CPlayerGun*) each frame. If Load() silently failed (OOM), IsLoaded()
    // never returns true and the beam transition hangs forever.
    //
    // Intercept the beq at 0x8003AF40 (branch-if-not-loaded to the bottom of the
    // state machine). Replace with bl stub. If heap has enough room, call Load()
    // via vtable[0x38] to retry the allocation; either way bail to the bottom so
    // the state machine retries next frame.
    //
    // Register state at intercept: r28=CPlayerGun*, r29=CStateManager*,
    // CR0 set by IsLoaded() result (EQ=1 if not loaded, EQ=0 if loaded).
    // LR = intercept_addr+4 (SetupBeam path) after bl replaces the beq.
    let intercept_addr: u32 = 0x8003AF40;
    let bottom_addr: u32 = 0x8003B02C;
    let free_bytes_addr: u32 = 0x804BFDF4;
    let threshold: u32 = 500 * 1024;

    let stub_addr = emitter.emit_addressed(dol_patcher, "beam_load_retry_stub", |cave_addr| {
        ppcasm!(cave_addr, {
            bne  is_loaded;
            lis  r12, { free_bytes_addr }@h;
            lwz  r0,  { free_bytes_addr }@l(r12);
            lis  r12, { threshold }@h;
            cmplw r0, r12;
            blt  no_retry;
            // Enough memory: retry Load() via vtable[0x38].
            // Save/restore LR (in LR register) around bctrl which clobbers it.
            stwu r1, -0x10(r1);
            mflr r0;
            stw  r0, 0x14(r1);
            lwz  r3, 0x734(r28);    // loadingBeam (non-null by earlier guard)
            lwz  r12, 0x0(r3);      // vtable
            mr   r4, r29;           // CStateManager
            li   r5, 0x0;
            lwz  r12, 0x38(r12);    // Load() = vtable[0x38]
            mtctr r12;
            bctrl;
            lwz  r0, 0x14(r1);
            mtlr r0;
            addi r1, r1, 0x10;
        no_retry:
            b    { bottom_addr };   // bail; state machine retries next frame
        is_loaded:
            blr;                    // loaded: return to SetupBeam path
        })
        .encoded_bytes()
    })?;
    dol_patcher.ppcasm_patch(&ppcasm!(intercept_addr, {
        bl { stub_addr };
    }))?;

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

fn patch_bss_heap_extension(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    if version != Version::NtscU0_00 {
        return Ok(());
    }

    // Inject the 80KB BSS gap at 0x80577BAC-0x8058BBAC into the heap free pool.
    // This region sits between CMetroidAreaCollider::sDupVertexList and sDupEdgeList
    // and is never accessed by the game binary. We hook the epilogue blr of
    // CGameAllocator::Initialize to construct a standalone free block chain
    // (head -> tail sentinel) and call AddFreeEntryToFreeList to register it.
    // Both addresses must be 32-byte aligned: SGameMemInfo pointer fields use & ~31 masking.
    // Raw gap: [0x80577BAC, 0x8058BBAC); snap inward to 32-byte boundaries.
    let head_addr: u32 = 0x8057_7BC0; // 0x80577BAC rounded up to 32-byte boundary
    let tail_addr: u32 = 0x8058_BB80; // last 32-byte boundary where tail + 0x20 <= 0x8058BBAC
    let game_allocator_addr: u32 = 0x804B_FD64;
    let heap_counter_addr: u32 = 0x804B_FDF4; // gGameAllocator + 0x90 = x90_heapSize2
    let add_free_entry_addr = symbol_addr!(
        "AddFreeEntryToFreeList__14CGameAllocatorFPQ214CGameAllocator12SGameMemInfo",
        version
    );
    let init_blr_addr = symbol_addr!("Initialize__14CGameAllocatorFR10COsContext", version) + 0x384;

    let cave_addr =
        emitter.emit_addressed(dol_patcher, "bss_heap_extension_stub", |cave_addr| {
            ppcasm!(cave_addr, {
                // CGameAllocator::Initialize already restored its own stack frame
                // (addi r1,r1,0x80) before the blr we replaced, so r1 is at the
                // caller's frame. We allocate a mini-frame for our bl below.
                stwu r1, -0x10(r1);
                mflr r0;
                stw  r0, 0x14(r1);

                // Build head SGameMemInfo at head_addr
                lis  r12, { head_addr }@h;
                addi r12, r12, { head_addr }@l;
                lis  r0, 0xEFEF;
                ori  r0, r0, 0xEFEF;
                stw  r0, 0x00(r12);        // x0_priorGuard = 0xEFEFEFEF
                lis  r0, 0x0001;
                ori  r0, r0, 0x3FA0;
                stw  r0, 0x04(r12);        // x4_len = 0x13FA0 (tail_addr - head_addr - 0x20)
                li   r0, 0;
                stw  r0, 0x08(r12);        // x8_fileAndLine = 0
                stw  r0, 0x0C(r12);        // xc_type = 0
                stw  r0, 0x10(r12);        // x10_prev = 0 (not allocated, no prior block)
                // addi with r0 as destination ignores r0's value (PPC r0 special case);
                // use lis+ori (zero-extend, no r0 exception) with plain upper bits.
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
                lis  r6, 0x0001;
                ori  r6, r6, 0x3FA0;       // r6 = 0x13FA0
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
    // if !config.qol_game_breaking {
    //     return Ok(());
    // }

    /* Patch heap allocator to tolerate failed allocations (return nullptr instead of panic) */
    patch_alloc_null_on_failure(dol_patcher, version)?;
    patch_alloc_oom_fast_fail(dol_patcher, version)?;
    patch_add_puddle_null_guard(dol_patcher, emitter, version)?;

    /* Patch to deny memory-hungry actions if heap below danger threshold (approx. 400KB - 700KB) */
    patch_morph_transition_oom_guard(dol_patcher, emitter, version)?;
    patch_map_open_oom_guard(dol_patcher, emitter, version)?;
    patch_change_weapon_oom_guard(dol_patcher, emitter, version)?;

    /* Retry beam load each frame until it succeeds (handles OOM mid-load stuck state) */
    patch_beam_load_retry(dol_patcher, emitter, version)?;

    /* Inject unused 80KB BSS gap into heap free pool (NTSC 0-00 only) */
    patch_bss_heap_extension(dol_patcher, emitter, version)?;

    /* Patch alloc call sites to tolerate nullptr return values */
    patch_build_async_null_guard(dol_patcher, emitter, version)?; // Solves start menu crash
    patch_inflate_null_guard(dol_patcher, emitter, version)?;
    patch_init_bitmap_buffers_null_guard(dol_patcher, emitter, version)?;

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

    let overflow_bytes = emitter.serialize_overflow();
    *file = structs::FstEntryFile::ExternalFile(Box::new(dol_patcher));
    Ok(overflow_bytes)
}
