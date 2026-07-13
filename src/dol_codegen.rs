// Machinery for injecting new PPC code into the DOL: version-aware symbol lookup, code-cave
// allocation, position-independent overflow stubs, and the TextEmitter that ties them together.
// Sits on top of DolPatcher (the DOL binary-format layer); consumed by dol_patches.rs.

use std::borrow::Cow;

use crate::dol_patcher::DolPatcher;
use crate::patch_config::Version;

macro_rules! symbol_addr {
    ($sym:tt, $version:expr) => {{
        let s = dol_symbol_table::mp1_symbol!($sym);
        match &$version {
            $crate::patch_config::Version::NtscU0_00 => s.addr_0_00,
            $crate::patch_config::Version::NtscU0_01 => s.addr_0_01,
            $crate::patch_config::Version::NtscU0_02 => s.addr_0_02,
            $crate::patch_config::Version::NtscK => s.addr_kor,
            $crate::patch_config::Version::NtscJ => s.addr_jpn,
            $crate::patch_config::Version::Pal => s.addr_pal,
            $crate::patch_config::Version::NtscUTrilogy => unreachable!(),
            $crate::patch_config::Version::NtscJTrilogy => unreachable!(),
            $crate::patch_config::Version::PalTrilogy => unreachable!(),
        }
        .unwrap_or_else(|| panic!("Symbol {} unknown for version {}", $sym, $version))
    }};
}
pub(crate) use symbol_addr;

// Non-panicking symbol lookup: None when the symbol is absent for the given version, so
// memory-optimization patches no-op on versions that don't yet name the function they hook.
macro_rules! symbol_addr_opt {
    ($sym:tt, $version:expr) => {{
        let s = dol_symbol_table::mp1_symbol!($sym);
        match &$version {
            $crate::patch_config::Version::NtscU0_00 => s.addr_0_00,
            $crate::patch_config::Version::NtscU0_01 => s.addr_0_01,
            $crate::patch_config::Version::NtscU0_02 => s.addr_0_02,
            $crate::patch_config::Version::NtscK => s.addr_kor,
            $crate::patch_config::Version::NtscJ => s.addr_jpn,
            $crate::patch_config::Version::Pal => s.addr_pal,
            $crate::patch_config::Version::NtscUTrilogy
            | $crate::patch_config::Version::NtscJTrilogy
            | $crate::patch_config::Version::PalTrilogy => None,
        }
    }};
}
pub(crate) use symbol_addr_opt;

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

struct HeapOverflowStub {
    dol_patch_site: u32,
    patch_is_bl: bool,
    stub_bytes: Vec<u8>,
    relocs: Vec<StubReloc>,
}

struct CodeCave {
    start: u32,
    size: u32,
    used: u32,
}

impl CodeCave {
    fn new(start: u32, size: u32) -> Self {
        CodeCave {
            start,
            size,
            used: 0,
        }
    }
    fn remaining(&self) -> u32 {
        self.size - self.used
    }
    fn alloc(&mut self, bytes: u32) -> u32 {
        let addr = self.start + self.used;
        self.used += bytes;
        addr
    }
}

pub(crate) struct CodeCaveAllocator {
    caves: Vec<CodeCave>,
}

impl CodeCaveAllocator {
    fn new(caves: Vec<CodeCave>) -> Self {
        CodeCaveAllocator { caves }
    }

    // Pick the smallest cave that still has room
    fn alloc(&mut self, bytes: u32) -> Option<u32> {
        let idx = self
            .caves
            .iter()
            .enumerate()
            .filter(|(_, c)| c.remaining() >= bytes)
            .min_by_key(|(_, c)| c.remaining())
            .map(|(i, _)| i)?;
        Some(self.caves[idx].alloc(bytes))
    }

    fn alloc_from_cave_start(&mut self, cave_start: u32, bytes: u32) -> u32 {
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

pub(crate) struct TextEmitter {
    cave_alloc: CodeCaveAllocator,
    overflow_stubs: Vec<HeapOverflowStub>,
}

impl TextEmitter {
    pub(crate) fn new(cave_alloc: CodeCaveAllocator) -> Self {
        TextEmitter {
            cave_alloc,
            overflow_stubs: Vec::new(),
        }
    }

    pub(crate) fn emit_at_cave_start(
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

    pub(crate) fn emit_addressed<F>(
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
    pub(crate) fn emit_and_patch<F>(
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
    pub(crate) fn serialize_overflow(&self) -> Vec<u8> {
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

// Rewrites a cave stub to be position-independent so it can run from an arbitrary heap address:
// external b/bl expand one word into four (lis/ori/mtctr/bctr[l]), internal b/bc are re-encoded for
// the new layout, and stub-local address materializations become relocs for the REL loader.
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

    // opcode-18 (b/bl, AA=0) branch leaving the stub - the only kind that expands one word to four.
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
pub(crate) fn branch_target(instr: u32, pc: u32) -> u32 {
    let li_raw = instr & 0x03FF_FFFC;
    let li: i32 = if li_raw & 0x0200_0000 != 0 {
        li_raw as i32 - 0x0400_0000
    } else {
        li_raw as i32
    };
    (pc as i32 + li) as u32
}

pub(crate) fn caves_for_version(version: Version) -> CodeCaveAllocator {
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

pub(crate) struct RelLoaderSelection {
    pub(crate) cave_bytes: &'static [u8],
    pub(crate) cave_map_str: &'static str,
    pub(crate) cave_start: u32,
}

pub(crate) fn rel_loader_selection(version: Version) -> RelLoaderSelection {
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
