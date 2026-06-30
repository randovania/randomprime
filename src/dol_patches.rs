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

// Non-panicking symbol lookup: None when the symbol is absent for the given version, so
// memory-optimization patches no-op on versions that don't yet name the function they hook.
macro_rules! symbol_addr_opt {
    ($sym:tt, $version:expr) => {{
        let s = mp1_symbol!($sym);
        match &$version {
            Version::NtscU0_00 => s.addr_0_00,
            Version::NtscU0_01 => s.addr_0_01,
            Version::NtscU0_02 => s.addr_0_02,
            Version::NtscK => s.addr_kor,
            Version::NtscJ => s.addr_jpn,
            Version::Pal => s.addr_pal,
            Version::NtscUTrilogy | Version::NtscJTrilogy | Version::PalTrilogy => None,
        }
    }};
}

// Dummy base for compiling overflow stubs before their heap address is known. Must be above
// all DOL addresses (0x80xxxxxx) so any branch target < STUB_COMPILE_BASE reads as external.
const STUB_COMPILE_BASE: u32 = 0x81000000;

// A stub-local lis/(addi|ori) pair referencing a data word inside the stub. make_pic records these
// so the REL loader rewrites the immediates to the stub's heap address. Offsets are within the
// expanded stub. See make_pic and apply_cave_overflow.
struct StubReloc {
    lis_pos: u32,
    lo_pos: u32,
    data_off: u32,
    is_addi: bool, // true: addi (sign-extended low half); false: ori
}

pub struct HeapOverflowStub {
    dol_patch_site: u32,
    patch_is_bl: bool,
    stub_bytes: Vec<u8>,
    relocs: Vec<StubReloc>,
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

    pub fn alloc_from_cave_start(&mut self, cave_start: u32, bytes: u32) -> u32 {
        let cave = self
            .caves
            .iter_mut()
            .find(|c| c.start == cave_start)
            .unwrap_or_else(|| panic!("CodeCaveAllocator: no cave starts at 0x{:08x}", cave_start));
        assert!(
            cave.used == 0,
            "Code cave 0x{:08x} already consumed {} bytes",
            cave_start,
            cave.used,
        );
        assert!(
            cave.remaining() >= bytes,
            "Code cave 0x{:08x} has only {} bytes remaining",
            cave_start,
            cave.remaining(),
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
        cave_start: u32,
        bytes: Vec<u8>,
    ) -> Result<u32, String> {
        let addr = self
            .cave_alloc
            .alloc_from_cave_start(cave_start, bytes.len() as u32);
        dol_patcher.patch(addr, Cow::Owned(bytes))?;
        Ok(addr)
    }

    pub fn emit_addressed<F>(
        &mut self,
        dol_patcher: &mut DolPatcher<'_>,
        build: F,
    ) -> Result<u32, String>
    where
        F: Fn(u32) -> Vec<u8>,
    {
        // 0x80000000 keeps all game-symbol bl targets within opcode limits
        const PROBE_ADDR: u32 = 0x80000000;
        let probe = build(PROBE_ADDR);
        let len = probe.len() as u32;
        let addr = self
            .cave_alloc
            .alloc(len)
            .unwrap_or_else(|| panic!("CodeCaveAllocator: no cave fits {} bytes", len));
        let final_bytes = build(addr);
        debug_assert_eq!(
            final_bytes.len(),
            probe.len(),
            "stub size changed under final address"
        );
        dol_patcher.patch(addr, Cow::Owned(final_bytes))?;
        Ok(addr)
    }

    // Allocates from caves first; on overflow, builds a position-independent stub and defers
    // the DOL patch to the REL loader via serialize_overflow(). Pure-code stubs only (no data).
    pub fn emit_and_patch<F>(
        &mut self,
        dol_patcher: &mut DolPatcher<'_>,
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
                "stub size changed under final address"
            );
            dol_patcher.patch(cave_addr, Cow::Owned(final_bytes))?;
            let rel = (cave_addr as i64 - dol_patch_site as i64) as u64;
            let lk: u32 = if patch_is_bl { 1 } else { 0 };
            let branch = (0x48000000u32 | lk) | (rel & 0x03FF_FFFC) as u32;
            dol_patcher.patch(dol_patch_site, Cow::Owned(branch.to_be_bytes().to_vec()))?;
        } else {
            let raw_bytes = build(STUB_COMPILE_BASE);
            let (pic_bytes, relocs) = make_pic(&raw_bytes, STUB_COMPILE_BASE);
            self.overflow_stubs.push(HeapOverflowStub {
                dol_patch_site,
                patch_is_bl,
                stub_bytes: pic_bytes,
                relocs,
            });
        }
        Ok(())
    }

    // Serializes overflow stubs to cave_overflow.bin format (empty Vec if none).
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
            out.extend_from_slice(&(stub.relocs.len() as u32).to_be_bytes());
            for r in &stub.relocs {
                out.extend_from_slice(&r.lis_pos.to_be_bytes());
                out.extend_from_slice(&r.lo_pos.to_be_bytes());
                out.extend_from_slice(&r.data_off.to_be_bytes());
                out.extend_from_slice(&(r.is_addi as u32).to_be_bytes());
            }
        }
        out
    }
}

// Sign-extend the low `bits` bits of `value` to a 32-bit signed integer.
fn sign_extend(value: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((value << shift) as i32) >> shift
}

// Makes a cave stub position-independent so it can run from an arbitrary heap address. Returns the
// rewritten bytes and a relocation list.
//   - External b/bl (target into the DOL) become ctr-indirect branches, growing one instruction
//     into four and shifting everything after:
//       b  target -> lis r12, target@h; ori r12, r12, target@l; mtctr r12; bctr
//       bl target -> lis r0,  target@h; ori r0,  r0,  target@l; mtctr r0;  bctrl
//   - Internal b/bc branches are re-encoded for the post-expansion layout.
//   - Stub-local lis/(addi|ori) address materializations are recorded as relocations for the REL
//     loader (see apply_cave_overflow); external/constant ones are emitted verbatim.
fn make_pic(stub_bytes: &[u8], stub_base: u32) -> (Vec<u8>, Vec<StubReloc>) {
    fn enc_lis(rd: u32, imm: u16) -> u32 {
        (15u32 << 26) | (rd << 21) | u32::from(imm)
    }
    fn enc_ori(rs: u32, ra: u32, imm: u16) -> u32 {
        (24u32 << 26) | (rs << 21) | (ra << 16) | u32::from(imm)
    }
    fn enc_mtctr(rs: u32) -> u32 {
        (31u32 << 26) | (rs << 21) | (0x120 << 11) | (467 << 1)
    }
    const BCTR: u32 = 0x4E80_0420;
    const BCTRL: u32 = 0x4E80_0421;

    assert_eq!(stub_bytes.len() % 4, 0);
    let n = stub_bytes.len() / 4;
    let stub_size = stub_bytes.len() as u32;
    let instrs: Vec<u32> = (0..n)
        .map(|k| u32::from_be_bytes(stub_bytes[k * 4..k * 4 + 4].try_into().unwrap()))
        .collect();

    // True iff instruction k is an opcode-18 (b/bl, AA=0) branch leaving the stub - the only kind
    // that expands from one word to four.
    let is_external_b = |k: usize| -> bool {
        let instr = instrs[k];
        if (instr & 0xFC00_0002) == 0x4800_0000 {
            let off = sign_extend(instr & 0x03FF_FFFC, 26);
            let target = (stub_base as i64 + (k as i64) * 4 + off as i64) as u32;
            target < stub_base || target >= stub_base + stub_size
        } else {
            false
        }
    };

    // Pass 1: byte offset each source instruction lands at after expansions.
    let mut new_off = vec![0u32; n + 1];
    for k in 0..n {
        new_off[k + 1] = new_off[k] + if is_external_b(k) { 16 } else { 4 };
    }

    // Pass 2: emit.
    let mut out: Vec<u8> = Vec::with_capacity(new_off[n] as usize);
    let mut relocs: Vec<StubReloc> = Vec::new();
    let mut k = 0usize;
    while k < n {
        let instr = instrs[k];
        let opcode = instr >> 26;
        let aa = (instr >> 1) & 1;

        // lis rX, A@h ; (addi|ori) rX, rX, A@l -> materialize an absolute address.
        if opcode == 15 && ((instr >> 16) & 0x1F) == 0 && k + 1 < n {
            let rt = (instr >> 21) & 0x1F;
            let next = instrs[k + 1];
            let nrt = (next >> 21) & 0x1F;
            let nra = (next >> 16) & 0x1F;
            let is_addi = (next >> 26) == 14 && nrt == rt && nra == rt;
            let is_ori = (next >> 26) == 24 && nrt == rt && nra == rt;
            if is_addi || is_ori {
                let hi = instr & 0xFFFF;
                let lo = next & 0xFFFF;
                let addr = if is_addi {
                    ((hi << 16) as i64 + sign_extend(lo, 16) as i64) as u32
                } else {
                    (hi << 16) | lo
                };
                if addr >= stub_base && addr < stub_base + stub_size {
                    let data_idx = ((addr - stub_base) / 4) as usize;
                    let lis_pos = out.len() as u32;
                    out.extend_from_slice(&instr.to_be_bytes());
                    let lo_pos = out.len() as u32;
                    out.extend_from_slice(&next.to_be_bytes());
                    relocs.push(StubReloc {
                        lis_pos,
                        lo_pos,
                        data_off: new_off[data_idx],
                        is_addi,
                    });
                    k += 2;
                    continue;
                }
            }
        }

        // opcode 18: b/bl (AA=0).
        if opcode == 18 && aa == 0 {
            let off = sign_extend(instr & 0x03FF_FFFC, 26);
            let target = (stub_base as i64 + (k as i64) * 4 + off as i64) as u32;
            if target < stub_base || target >= stub_base + stub_size {
                let lk = instr & 1;
                let hi = (target >> 16) as u16;
                let lo = (target & 0xFFFF) as u16;
                if lk == 0 {
                    out.extend_from_slice(&enc_lis(12, hi).to_be_bytes());
                    out.extend_from_slice(&enc_ori(12, 12, lo).to_be_bytes());
                    out.extend_from_slice(&enc_mtctr(12).to_be_bytes());
                    out.extend_from_slice(&BCTR.to_be_bytes());
                } else {
                    out.extend_from_slice(&enc_lis(0, hi).to_be_bytes());
                    out.extend_from_slice(&enc_ori(0, 0, lo).to_be_bytes());
                    out.extend_from_slice(&enc_mtctr(0).to_be_bytes());
                    out.extend_from_slice(&BCTRL.to_be_bytes());
                }
            } else {
                // internal: re-encode the offset for the post-expansion layout.
                let t = ((target - stub_base) / 4) as usize;
                let new_rel = new_off[t] as i64 - new_off[k] as i64;
                let field = (new_rel as i32 as u32) & 0x03FF_FFFC;
                out.extend_from_slice(&((instr & 0xFC00_0003) | field).to_be_bytes());
            }
            k += 1;
            continue;
        }

        // opcode 16: bc (conditional, AA=0). Always internal in our stubs.
        if opcode == 16 && aa == 0 {
            let off = sign_extend(instr & 0x0000_FFFC, 16);
            let target = (stub_base as i64 + (k as i64) * 4 + off as i64) as u32;
            assert!(
                target >= stub_base && target < stub_base + stub_size,
                "external conditional branch in overflow stub is not supported"
            );
            let t = ((target - stub_base) / 4) as usize;
            let new_rel = new_off[t] as i64 - new_off[k] as i64;
            let field = (new_rel as i32 as u32) & 0x0000_FFFC;
            out.extend_from_slice(&((instr & !0x0000_FFFC) | field).to_be_bytes());
            k += 1;
            continue;
        }

        out.extend_from_slice(&instr.to_be_bytes());
        k += 1;
    }
    (out, relocs)
}

// Decodes the absolute target of a PPC I-form branch (b/bl, AA=0) at `pc`. Used to recover a
// call target when the callee's mangled name differs across versions (symbol lookup not portable).
fn branch_target(instr: u32, pc: u32) -> u32 {
    let li_raw = instr & 0x03FF_FFFC;
    let li: i32 = if li_raw & 0x0200_0000 != 0 {
        li_raw as i32 - 0x0400_0000
    } else {
        li_raw as i32
    };
    (pc as i32 + li) as u32
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

// Eliminate the 1-2s freeze (and rumble buzz) when CGameAllocator::Alloc can't find a block. Two
// costs follow `beq skip_callback` at +0x284: the OOM callback (~1s ARAM-DMA spin with interrupts
// off) and DumpAllocations (~0.5s of throttled block iteration). Replacing the beq with `b +0xA0`
// jumps to the `li r3,0` null return, skipping both. Offsets identical across versions.
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

// Null-guard CResFactory::BuildAsync: on a null alloc (e.g. fragmentation during CSamusDoll
// construction) return early leaving *ppObj == null, so IsLoaded() is false and Draw() (already
// IsLoaded-guarded) skips rendering. Requires patch_alloc_null_on_failure first.
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

    // Frame 0x70: 0x74(r1) = saved LR; epilogue at +0xC8 restores and returns.
    let cave_addr = emitter.emit_addressed(dol_patcher, |cave_addr| {
        ppcasm!(cave_addr, {
            cmpwi r29, 0x0;                                // null alloc result?
            bne do_load;
            b {build_async_addr + 0xC8};                   // early return via epilogue
        do_load:
            mr r4, r26;                                    // displaced originals:
            addi r3, r25, 0x4;
            mr r5, r29;
            b {load_resource_async_addr};                  // tail call; LR = return addr of bl cave
        })
        .encoded_bytes()
    })?;

    // BuildAsync+0x6C: [mr r4,r26 | addi r3,r25,4 | mr r5,r29 | bl LoadResourceAsync] becomes
    // [bl cave | b +0x7C | nop | nop]. LoadResourceAsync returns to +0x70; the b +0x7C skips the
    // dead instrs to the mr r30,r3 that consumes the return value.
    dol_patcher.ppcasm_patch(&ppcasm!(build_async_addr + 0x6C, {
        bl {cave_addr};
        b {build_async_addr + 0x7C};
        nop;
        nop;
    }))?;

    Ok(())
}

// Retry instead of crash when the inflate output buffer (SLD_inner[0x20]) is null on OOM (the loop
// would pass it as z_stream.next_out and crash). The stub intercepts the load and, on null, calls
// inflateEnd, clears the z_stream reference (56-byte struct leaked, one per OOM), and returns 0 via
// the failure epilogue; AsyncIdle retries next frame. Gated to PumpResource + inflateEnd.
fn patch_inflate_null_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    // The zlib decompress worker is unnamed; recover its start by decoding the bl at
    // PumpResource+0x5c (its sole caller). Worker offsets verified identical across versions.
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

    // r30 = SLD_inner, r23 = z_stream ptr (set up before the loop). Non-null: return r3 via blr.
    // Null (OOM): inflateEnd, clear the z_stream reference, set oom_flag, jump to the epilogue;
    // the next retry re-allocates z_stream + buf fresh. r12 is reloaded fresh (inflateEnd clobbers).
    let stub_addr = emitter.emit_addressed(dol_patcher, |cave_addr| {
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

// Null-guard the medium-pool expansion in CGameAllocator::Alloc. To grow the pool, Alloc recurses
// for a block and passes it to CMediumAllocPool::AddPuddle; with patch_alloc_null_on_failure that
// can be null, and AddPuddle would deref it (0 + capacity*32) and crash. On null we skip AddPuddle
// and fall through to the pool-retry path, which fails again and returns null. Gated by symbol.
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

    // Expansion sequence at Alloc+0x1c4 (offset identical across versions): the 5 instrs setting
    // up AddPuddle, then `bl AddPuddle`. AddPuddle's mangled name differs by version (extra param
    // on 1.02/PAL), so decode the branch target instead of looking the symbol up.
    let intercept_addr: u32 = alloc + 0x1c4;
    let add_puddle_addr: u32 = branch_target(
        dol_patcher.read_u32(intercept_addr + 0x14)?,
        intercept_addr + 0x14,
    );
    let retry_addr: u32 = intercept_addr + 0x18; // CMediumAllocPool::Alloc retry after AddPuddle

    // r3 = inner alloc result (null on OOM). Non-null: replay the 5 setup instrs and tail-call
    // AddPuddle. Null: blr, skipping AddPuddle. Both return to LR=intercept+4 (a b {retry_addr}).
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

// Null-guard CTexture::InitBitmapBuffers: on a null bitmap-buffer alloc (OOM during beam-switch
// or large texture load) skip PostConstruct + CountMemory and jump to the epilogue, leaving
// CARAMToken default-constructed (state=6); LoadToARAM returns 0 early for state==6, so nothing
// downstream crashes. Requires patch_alloc_null_on_failure. Gated to versions that name it.
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

// Completes patch_init_bitmap_buffers_null_guard. With the null MRAM buffer that guard leaves, the
// CTexture ctor's read loop (which runs before any LoadToARAM) memcpys into address 0 and crashes.
// The stub replays the displaced `mr r28, r3` and, on null, skips the read + MangleMipmap loops to
// InitTextureObjects (CPU-side setup, never touches the buffer); texture renders blank. Gated by symbol.
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

// Reduce peak heap during a beam switch by freeing the outgoing beam earlier. The gun morph holds
// both beams at once (new loaded in ChangeWeapon, old freed only at ProcessGunMorph kGS_OutWipeDone);
// an area transition in that window can drive free heap to zero and crash a later scratch alloc
// (threshold guards don't help -- the drop is the post-switch area load, not the switch).
//
// After the swap (kGS_InWipeDone) only the new beam (x72c) is drawn; the outgoing beam (x730) is held
// but never read until OutWipeDone (verified). So we replicate the OutWipeDone unload right after the
// swap by replacing the swap case's closing `b` with `bl stub`; OutWipeDone then finds x730 null and
// skips (no double free). Gated by symbol.
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
            // outgoingBeam->Unload(mgr) via vtable[0x3c]. Exit via `b` not blr, so the
            // bctrl-clobbered LR is irrelevant (mirrors the OutWipeDone code).
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

    // DrawAreas renders the automap every frame via a local vector whose reserve(), under OOM,
    // leaves a null backing pointer while size is bumped -> next store crashes. A per-store guard
    // won't do (the sort/draw pass derefs the same buffer), so under low heap we skip the whole
    // function: it returns void, is per-frame and stack-only, so it self-corrects when heap recovers.
    // b-trampoline (LR untouched), silent; reserve is tiny so the floor only blanks near zero heap.
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

    // ChangeWeapon+0x78 is `beq +0xa4` (skip beam Load when loading==current). Replace it with a
    // stub that honours the original beq, then on OOM jumps to the epilogue (+0xf4), skipping the
    // wipe so the morphing flag is never set and the player can still fire. Offsets identical across
    // versions. CR0[EQ] survives bl; r0/r12 are dead (overwritten at +0x7c).
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
            // OOM: HandleBeamChange already set x2f8 |= 0x8 (morphing), which makes ProcessInput
            // exit early and the player can't fire. The flag is normally cleared after a successful
            // morph, which never happens here, so restore x2f8 = 1 (beam mode) ourselves.
            li   r0, 0x1;
            stw  r0, 0x2f8(r29);
            // Play malfunction SFX. Push a mini-frame for the CSfxHandle out-param (SfxStart writes
            // r3); the target epilogue restores LR from the saved frame, so clobbering LR here is
            // fine. Pop before the jump so ChangeWeapon's frame is back on top.
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
    logbook_build_flag: u32,
) -> Result<(), String> {
    let (Some(alloc), Some(func)) = (
        symbol_addr_opt!("gGameAllocator", version),
        symbol_addr_opt!(
            "BuildPauseSubScreen__12CPauseScreenFQ212CPauseScreen10ESubScreenRC13CStateManagerRC9CGuiFrame",
            version
        ),
    ) else {
        return Ok(());
    };

    // BuildPauseSubScreen's Log Book case (func+0x4c) news a CLogBookScreen whose ctor synchronously
    // builds every world pak directory and locks their scan tokens with no retry. The world-pak guard
    // would DEFER those small dir builds (its contiguous-free floor is calibrated for gameplay
    // world-loads, not these), and with no retry that leaves a pak's scans unloadable -> null CPakFile
    // deref. So set logbook_build_flag here to make that guard FORCE the build (vanilla behaviour);
    // the flag is cleared at the post-build join (func+0x80). We still deny on genuinely low TOTAL
    // free: BuildPauseSubScreen returns null on alloc failure and StartTransition tolerates it (return
    // null via func+0x108; the tab just doesn't populate).
    //
    // b-trampoline: all volatiles (r0, r3-r12, CR0) are dead in this case; displaced lis replayed;
    // threshold kept 0x10000-aligned (loaded via lis only).
    let free_bytes_addr: u32 = alloc + 0x90;
    let total_threshold: u32 = 2304 * 1024;
    let intercept_addr: u32 = func + 0x4c;
    let return_null_addr: u32 = func + 0x108;

    let intercept_orig = dol_patcher.read_u32(intercept_addr)?;
    emitter.emit_and_patch(dol_patcher, intercept_addr, false, |cave_addr| {
        ppcasm!(cave_addr, {
            lis   r12, { free_bytes_addr }@h;
            lwz   r0,  { free_bytes_addr }@l(r12);
            lis   r12, { total_threshold }@h;
            cmplw r0, r12;
            blt   oom;                        // genuinely low total free -> deny the page
            li    r0, 1;                      // logbook opening -> force world-pak dir builds
            lis   r12, { logbook_build_flag }@h;
            stw   r0,  { logbook_build_flag }@l(r12);
            .long intercept_orig;             // trampoline: original first instruction (lis r4, ...)
            b     { intercept_addr + 4 };
        oom:
            b     { return_null_addr };       // return null subscreen, skip log book
        })
        .encoded_bytes()
    })?;

    // Clear the flag at the Log Book case's post-build join (func+0x80, `mr r3, r0`). Replay that
    // first so r0 is free to zero the flag; r12 is dead here.
    let clear_addr: u32 = func + 0x80;
    let clear_orig = dol_patcher.read_u32(clear_addr)?;
    emitter.emit_and_patch(dol_patcher, clear_addr, false, |cave_addr| {
        ppcasm!(cave_addr, {
            .long clear_orig;                 // mr r3, r0 (screen ptr -> return value)
            li    r0, 0;
            lis   r12, { logbook_build_flag }@h;
            stw   r0,  { logbook_build_flag }@l(r12);
            b     { clear_addr + 4 };
        })
        .encoded_bytes()
    })?;

    Ok(())
}

fn patch_world_pak_ready_oom_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
    logbook_build_flag: u32,
) -> Result<(), String> {
    let (Some(func_addr), Some(get_largest_free_chunk), Some(game_allocator)) = (
        symbol_addr_opt!("EnsureWorldPakReady__8CPakFileFv", version),
        symbol_addr_opt!("GetLargestFreeChunk__14CGameAllocatorCFv", version),
        symbol_addr_opt!("gGameAllocator", version),
    ) else {
        return Ok(());
    };

    // EnsureWorldPakReady builds a pak's resource directory via a reserve that, under OOM, leaves a
    // null backing pointer while size increments -> crash. It reads GetLargestFreeChunk, not the x90
    // total: reserve needs one contiguous block, and pause-menu churn fragments the heap so total free
    // can pass a threshold while no chunk fits. The build is deferrable (needs-build bit cleared only
    // at the end), so on a too-small chunk we return without building and the next pass retries.
    //
    // EXCEPTION: while logbook_build_flag is set (see patch_logbook_oom_guard) we FORCE the build,
    // since the logbook locks scans immediately with no retry and a deferral there is a null deref.
    //
    // b-trampoline. The flag check uses r0/r12 (dead at entry) so the force-build path needs no
    // mini-frame; the defer path uses one (the bl clobbers LR and r3 must survive). Keep threshold
    // 0x10000-aligned; displaced stwu replayed.
    let threshold: u32 = 448 * 1024;

    let func_orig = dol_patcher.read_u32(func_addr)?;
    emitter.emit_and_patch(dol_patcher, func_addr, false, |cave_addr| {
        ppcasm!(cave_addr, {
            lis   r12, { logbook_build_flag }@h;
            lwz   r0,  { logbook_build_flag }@l(r12);
            cmpwi r0, 0;
            bne   build;                     // logbook building -> force the build, skip deferral
            stwu  r1, -0x10(r1);             // mini-frame to survive the bl
            mflr  r0;
            stw   r0, 0x14(r1);              // save caller LR (b-trampoline left it in LR)
            stw   r3, 0x10(r1);              // save incoming CPakFile* this
            lis   r3, { game_allocator }@h;
            addi  r3, r3, { game_allocator }@l;// r3 = &gGameAllocator
            bl    { get_largest_free_chunk };// r3 = largest contiguous free bytes
            lis   r12, { threshold }@h;
            cmplw r3, r12;                   // unsigned compare vs contiguous floor
            lwz   r0, 0x14(r1);              // restore caller LR
            mtlr  r0;
            lwz   r3, 0x10(r1);              // restore this ptr
            addi  r1, r1, 0x10;              // pop mini-frame (CR0 from cmplw preserved)
            blt   oom;
        build:
            .long func_orig;                 // trampoline: original first instruction
            b     { func_addr + 4 };
        oom:
            blr;                             // defer build; needs-build bit stays set, retried
        })
        .encoded_bytes()
    })?;

    Ok(())
}

// Defer (rather than broken-build) async resource construction when heap is too low -- the
// root-cause fix for low-memory beam-switch failures. The null-guards above stop a mid-build alloc
// failure from crashing, but the resource finishes degraded and PumpResource caches it as "loaded";
// nothing re-pumps it, so a beam built in a transient low-heap window stays broken and the morph
// hangs even back in a memory-rich area (broken cache, not capacity). PumpResource already defers a
// not-ready resource (returns 0, node stays queued); we extend that to a ready resource when free
// heap is below threshold, so the heavy build waits for memory and caches cleanly.
//
// Time-bounded: the loader drops a build's read data after ~5s and orphans the CObjectReference
// forever, so a continuous low-heap stall past DEFER_TIMEOUT_TICKS (4.0s) forces the build through
// (worst case degraded, not wedged). The stall is timed via OSGetTime in one global scratch slot;
// the floor is global so every pump defers-all or proceeds-all, and the first proceed (or forced
// build) clears the timer -- it only accumulates across continuous low heap.
//
// Must NOT defer on the synchronous Build path (spins until nonzero); it's distinguished by the
// time-budget arg saved to r31 (Build = 0, AsyncIdle != 0), so only defer when r31 != 0. Replace
// the readiness beq with `bl stub`: CR0/scratch/nonvolatiles all survive, LR is frame-saved (stub
// exits via `b`). Gated to PumpResource.
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
        // Cave is >32KB from PumpResource, so conditionals target local labels and far jumps use b.
        ppcasm!(cave_addr, {
            beq   defer;                 // original: resource not ready -> defer (no timeout)
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

// Recover the resource decompressor from a failed output-buffer alloc instead of wedging. The worker
// sets up a zlib stream once then allocates the output buffer; under fragmentation the total-free
// defer guard can pass yet this contiguous alloc returns null. The original code then loops on a null
// buffer (non-crashing only thanks to patch_inflate_null_guard) producing 0 bytes, and every later
// call resumes the already-set-up stream into the same null buffer forever -- the beam-morph wedge.
//
// b-trampoline the `neg r0, r3` after the Alloc: non-null runs the original; null tears the stream
// down like the worker's own cleanup (inflateEnd, then Free when owned), clears it, and returns 0 via
// the failure epilogue, so the next pump re-allocates and recovers. Fires only on real alloc failure,
// so it never over-defers. r23/r30 survive the calls; stub runs in the worker's frame and exits via b.
fn patch_inflate_buffer_oom_recover(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
) -> Result<(), String> {
    // Same unnamed worker as patch_inflate_null_guard: recover its start via the bl at
    // PumpResource+0x5c. Worker offsets verified identical across versions.
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

    // Shared flag the logbook guard sets so the world-pak-ready guard force-builds rather than defers.
    // Reserved here so both guards see the same address.
    let logbook_build_flag =
        emitter.emit_addressed(dol_patcher, |_| 0u32.to_be_bytes().to_vec())?;

    /* Patch heap allocator to tolerate failed allocations (return nullptr instead of panic) */
    patch_alloc_null_on_failure(dol_patcher, version)?;
    patch_alloc_oom_fast_fail(dol_patcher, version)?;
    patch_add_puddle_null_guard(dol_patcher, emitter, version)?;

    /* Patch to deny memory-hungry actions if heap below danger threshold */
    patch_morph_transition_oom_guard(dol_patcher, emitter, version)?;
    patch_draw_areas_oom_guard(dol_patcher, emitter, version)?;
    patch_logbook_oom_guard(dol_patcher, emitter, version, logbook_build_flag)?;
    patch_world_pak_ready_oom_guard(dol_patcher, emitter, version, logbook_build_flag)?;
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

// ============================================================================
//  RANDOMPRIME SAVE-SLOT PERSISTENT DATA - SCHEMA (single source of truth)
// ============================================================================
// randomprime stores its own data in the TAIL of every CGameState save buffer,
// past the game's own serialized content. The constants below are authoritative:
// the Rust block builder (build_save_schema_block) and the PPC trampolines
// (stamp / block / name) both derive every offset from them, so the two sides
// cannot drift, and the compile-time asserts fail the build if the block stops
// fitting. See doc/dol-patching.md.
//
//   off  size  field      notes
//   0    4     magic      SAVE_SCHEMA_MAGIC ("RPSV"); absent => not our save
//   4    4     version    SAVE_SCHEMA_VERSION (current = 1)
//   8    16    uuid       instance id; all-zero default
//   24   36    save_name  big-endian UTF-16, null-terminated (<= 17 chars)
//   60   36    reserved   zeroed; future fields append here (bump version)
//
// The block occupies SAVE_SCHEMA_OFFSET..SAVE_BUFFER_SIZE_MIN of the buffer.
// Reads are instance-agnostic (fixed offsets, independent of CPlayerState's
// serialized width / itemMaxCapacity), so instances with different configs read
// each other's data. PutTo entry stamps the block (patch_save_uuid_stamp),
// StartGame gates loads on magic+uuid (patch_save_uuid_block), and the
// file-select renders save_name (patch_save_name).

// Smallest save buffer across supported versions (NTSC-U / K = 0x3ac = 940). PAL
// (0xa7f = 2687) and NTSC-J (0x888 = 2184) over-allocate, but the pre-world-state
// serialized layout is byte-identical across versions and world-state content is
// language-independent, so max content is ~793 bytes everywhere and a uniform
// tail offset is safe for all. The runtime guard (patch_save_schema_guard) is the
// empirical backstop.
const SAVE_BUFFER_SIZE_MIN: i32 = 0x3ac; // 940

const SAVE_SCHEMA_SIZE: i32 = 96; // total reserved tail block
const SAVE_SCHEMA_OFFSET: i32 = SAVE_BUFFER_SIZE_MIN - SAVE_SCHEMA_SIZE; // 844

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

const MOUTPTR_OFF: i32 = 0x7c; // sizeof(COutputStream) = offset of CMemoryStreamOut::mOutPtr
const MNUMWRITES_OFF: i32 = 0x10; // COutputStream::mNumWrites (bytes flushed); used by the guard

// ---- Front "dead" region (scaffolded for future use; NOT used yet) ----
// CGameState's first serialized member is a 128-byte array (count hard-fixed to 128 at
// CGameState+0x0, inline data at +0x4) written 8 bits/byte = buffer bytes 0..128. It is
// ctor-zeroed and never read for gameplay, so it is reliably always-zero reserved space.
// A future patch may store data here without any engine change; documented so the address is
// known. See doc/dol-patching.md.
#[allow(dead_code)]
const SAVE_FRONT_OFFSET: i32 = 0;
#[allow(dead_code)]
const SAVE_FRONT_SIZE: i32 = 128;

const _: () = assert!(
    SAVE_SCHEMA_OFFSET + SAVE_SCHEMA_SIZE <= SAVE_BUFFER_SIZE_MIN,
    "save schema block overflows the smallest save buffer"
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

// Per-version layout for the save-uuid/name feature. All fields are struct or instruction offsets
// read directly from each build's disassembly; localized builds (PAL, NTSC-J) shift CGameState
// members so their offsets differ from the NTSC-U family.
struct SaveUuidLayout {
    start_hook_off: u32,     // StartGame: the `bl BuildNewFileSlot` load (block hook)
    start_epilogue_off: u32, // StartGame: epilogue (enter-game store skipped on deny)
    driver_off: i32,         // CSaveGameScreen -> CMemoryCardDriver (block)
    slot_table_off: i32,     // driver -> SGameFileSlot table (block)
    slot_buf_off: i32,       // SGameFileSlot -> raw save buffer (block)
    name_buf_off: i32,       // SetupFrameContents: GameFileStateInfo reg -> raw save buffer (name)
    setup_hook_off: u32,     // SetupFrameContents hook (`cmplwi <name>, 0`)
    // SetupFrameContents register allocation: false = NTSC/PAL (name ptr r25, GameFileStateInfo r26);
    // true = NTSC-J, which saves one fewer non-volatile so both shift up by one (r26 / r27).
    setup_regs_shifted: bool,
}

fn save_uuid_layout(version: Version) -> Option<SaveUuidLayout> {
    match version {
        // NTSC-U family + Korean share a byte-identical CGameState/CSaveGameScreen layout
        // (verified: CGameState::PutTo and SetupFrameContents disassemble identically).
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
        // PAL is a localized build whose CGameState/CMemoryCardDriver layout differs from the
        // NTSC-U family. SetupFrameContents matches NTSC's register allocation (name ptr r25,
        // GameFileStateInfo r26) but its save buffer is larger, so
        // name_buf_off = 0x4 - 0xa88 = -0xa84 (fileInfo at slot+0xa88).
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
        // NTSC-J: another localized build, with the largest layout shifts (CMemoryCardDriver slot
        // table 0x694; StartGame is still structurally identical to NTSC). SetupFrameContents saves
        // one fewer non-volatile, shifting its registers up by one (name ptr r26, GameFileStateInfo
        // r27) and placing the hook one instruction earlier (+0x1fc); fileInfo at slot+0x890 so
        // name_buf_off = 0x4 - 0x890 = -0x88c.
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

// Build the SAVE_SCHEMA_SIZE-byte schema block exactly as it lives in the save buffer tail. This is
// the Rust side of the single source of truth; the trampolines read the same offsets.
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

// Stamp the schema block into the save buffer tail on every CGameState::PutTo. Hooks PutTo entry to
// copy the whole SAVE_SCHEMA_SIZE-byte block from the cave directly to
// CMemoryStreamOut::mOutPtr + SAVE_SCHEMA_OFFSET, before the engine serializes its own content.
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
        // r3 = CGameState* (this), r4 = CMemoryStreamOut*; use r6/r9/r12 as scratch.
        ppcasm!(cave_addr, {
            lwz   r12, { MOUTPTR_OFF }(r4); // mOutPtr = raw save buffer base
            addi  r12, r12, { SAVE_SCHEMA_OFFSET }; // schema block base
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

// Block loading a save whose stamped UUID does not match this instance's.
//
// We hook StartGame's bl BuildNewFileSlot (the load): for an existing save, first check our magic
// at save_buf+SAVE_OFF_MAGIC (absent => not our save => allow), then compare the 4 UUID words and
// on mismatch skip the load, play a denied sfx, and branch to the epilogue (the enter-game store
// never runs, screen stays open). New files, non-randomprime saves, and matching saves load
// normally. EraseGame is untouched, so a foreign save stays deletable.
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

// On each file-select row, show the saveName stored in that slot's save buffer instead of the world name.
//
// SetupFrameContents resolves the world name into a `name` register then builds an rstl::wstring. We
// hook the `cmplwi <name>, 0` just before that build. The row's GameFileStateInfo* is in a `fileInfo`
// register; the name lives at fileInfo + name_buf_off + SAVE_NAME_FIXED_BYTE_OFF (a fixed offset into
// the save buffer tail). We copy the 9 name words into per-row scratch and point `name` at it; if the
// first word is zero (no stored name), `name` keeps the world name. The row index is r30 in every
// build; name/fileInfo are r25/r26 on NTSC+PAL and r26/r27 on NTSC-J (setup_regs_shifted).
// allow(dead_code): ppcasm's generated per-label struct trips the dead_code lint when the ppcasm! is
// wrapped in the local `name_tramp!` macro (a macro-hygiene quirk); the labels are used by branches.
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
        // $name = SetupFrameContents' world-name pointer register, $fileinfo = its GameFileStateInfo
        // register. Everything else uses scratch (r0, r3-r12) and the build-invariant row index r30.
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
// diagnostic hooks here.
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
    patch_save_schema_guard(dol_patcher, emitter, version)?;
    Ok(())
}

// Runtime half of "panic when the schema overflows": at CGameState::PutTo exit, compare the bytes
// the engine serialized (CMemoryStreamOut::mNumWrites) against SAVE_SCHEMA_OFFSET. If serialization
// ever reached our tail block, OSReport a loud line so a playtest surfaces the collision (the stamp
// hooks PutTo *entry* and can't see the final size, hence the exit hook).
fn patch_save_schema_guard(
    dol_patcher: &mut DolPatcher<'_>,
    emitter: &mut TextEmitter,
    version: Version,
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

    let mut fmt_bytes = b"randomprime: SAVE SCHEMA OVERFLOW wrote=%08x limit=%08x\n\0".to_vec();
    while fmt_bytes.len() % 4 != 0 {
        fmt_bytes.push(0);
    }
    let fmt_addr = emitter.emit_addressed(dol_patcher, move |_| fmt_bytes.clone())?;

    emitter.emit_and_patch(dol_patcher, epilogue, false, |cave_addr| {
        ppcasm!(cave_addr, {
            lwz   r4, { MNUMWRITES_OFF }(r31); // bytes serialized
            cmpwi r4, { SAVE_SCHEMA_OFFSET };
            blt   ok;                          // no collision
            lis   r3, { (fmt_addr >> 16) as i32 };
            ori   r3, r3, { (fmt_addr & 0xffff) as i32 };
            li    r5, { SAVE_SCHEMA_OFFSET };
            bl    { osreport };                // clobbers r0,r3-r12,LR,CTR; r31 + stack-saved LR survive
        ok:
            .long epilogue_orig;               // displaced `lmw r24, 0x30(r1)`
            b     { epilogue + 4 };
        })
        .encoded_bytes()
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

    // Emitted last for readability; these emit_and_patch stubs are overflow-safe (see make_pic).
    patch_save_uuid_stamp(&mut dol_patcher, &mut emitter, version, &save_uuid_data)?;
    patch_save_uuid_block(&mut dol_patcher, &mut emitter, version, &save_uuid_data)?;
    patch_save_name(&mut dol_patcher, &mut emitter, version, &save_uuid_data)?;

    let overflow_bytes = emitter.serialize_overflow();
    *file = structs::FstEntryFile::ExternalFile(Box::new(dol_patcher));
    Ok(overflow_bytes)
}
