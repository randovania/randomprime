#![no_std]

extern crate alloc;

use core::mem::MaybeUninit;

use linkme::distributed_slice;
use primeapi::{
    alignment_utils::Aligned32,
    dol_sdk::dvd::DVDFileInfo,
    mp1::{
        CArchitectureQueue, CGameState, CGuiFrame, CGuiTextPane, CGuiTextSupport, CGuiWidget,
        CMainFlow, CStringTable, CWorldState,
    },
    patch_fn, prolog_fn,
    rstl::WString,
    GameVersion,
};

// Flush dcache to RAM then invalidate icache so freshly-written heap code executes correctly.
// Uses hardcoded DOL addresses for the versions where they are known.
// Required for correctness on real hardware; Dolphin handles this transparently.
unsafe fn flush_and_invalidate(ptr: *const u8, len: usize) {
    type CacheFn = unsafe extern "C" fn(*const u8, u32);
    let (dc, ic): (u32, u32) = match GameVersion::current() {
        GameVersion::Ntsc0_00 => (0x8037_EAB0, 0x8037_EB94),
        GameVersion::Ntsc0_02 => (0x8037_F8AC, 0x8037_F990),
        _ => return,
    };
    let dc_flush: CacheFn = core::mem::transmute(dc);
    let ic_inval: CacheFn = core::mem::transmute(ic);
    dc_flush(ptr, len as u32);
    ic_inval(ptr, len as u32);
}

#[inline(always)]
fn read_u32_be(data: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

#[prolog_fn]
unsafe extern "C" fn apply_cave_overflow() {
    let mut fi = if let Some(fi) = DVDFileInfo::new(b"cave_overflow.bin\0") {
        fi
    } else {
        return;
    };
    let file_size = fi.file_length() as usize;
    if file_size < 8 {
        return;
    }

    let mut raw = alloc::vec![MaybeUninit::<u8>::uninit(); file_size + 63];
    let buf = Aligned32::split_unaligned_prefix_mut(&mut raw[..]).1;
    let buf = &mut buf[..(file_size + 31) & !31];
    { let _ = fi.read_async(buf, 0, 0); }
    // Safe: DVD read filled these bytes.
    let data: &[u8] = core::slice::from_raw_parts(buf.as_mut_ptr() as *const u8, file_size);

    let stub_count = read_u32_be(data, 0) as usize;
    let stubs_total = read_u32_be(data, 4) as usize;
    if stub_count == 0 || stubs_total == 0 {
        return;
    }

    // Must outlive this function — DOL branch sites point here for the game's lifetime.
    let stubs_buf = {
        let raw = primeapi::malloc(stubs_total + 31);
        ((raw as usize + 31) & !31) as *mut u8
    };

    let mut meta = 8usize;
    let mut buf_off = 0usize;

    for _ in 0..stub_count {
        if meta + 12 > data.len() {
            break;
        }
        let site = read_u32_be(data, meta);
        let is_bl = read_u32_be(data, meta + 4) != 0;
        let size = read_u32_be(data, meta + 8) as usize;
        meta += 12;

        if meta + size > data.len() || buf_off + size > stubs_total {
            break;
        }

        let heap_addr = stubs_buf as u32 + buf_off as u32;
        core::ptr::copy_nonoverlapping(data.as_ptr().add(meta), stubs_buf.add(buf_off), size);
        meta += size;
        buf_off += size;

        // Rewrite make_pic's stub-local lis/addi|ori relocations to the stub's heap address.
        if meta + 4 > data.len() {
            break;
        }
        let reloc_count = read_u32_be(data, meta) as usize;
        meta += 4;
        for _ in 0..reloc_count {
            if meta + 16 > data.len() {
                break;
            }
            let lis_pos = read_u32_be(data, meta);
            let lo_pos = read_u32_be(data, meta + 4);
            let data_off = read_u32_be(data, meta + 8);
            let is_addi = read_u32_be(data, meta + 12) != 0;
            meta += 16;

            let target = heap_addr.wrapping_add(data_off);
            let (hi, lo) = if is_addi {
                // addi sign-extends its imm; bias the high half when bit 15 is set.
                ((target.wrapping_add(0x8000) >> 16) & 0xFFFF, target & 0xFFFF)
            } else {
                ((target >> 16) & 0xFFFF, target & 0xFFFF)
            };
            let lis_ptr = (heap_addr + lis_pos) as *mut u32;
            let lo_ptr = (heap_addr + lo_pos) as *mut u32;
            core::ptr::write(lis_ptr, (core::ptr::read(lis_ptr) & 0xFFFF_0000) | hi);
            core::ptr::write(lo_ptr, (core::ptr::read(lo_ptr) & 0xFFFF_0000) | lo);
        }

        let rel = (heap_addr as i64 - site as i64) as u64;
        let lk: u32 = if is_bl { 1 } else { 0 };
        let branch = (0x4800_0000u32 | lk) | (rel & 0x03FF_FFFC) as u32;
        core::ptr::write(site as *mut u32, branch);
        flush_and_invalidate(site as *const u8, 4);
    }

    flush_and_invalidate(stubs_buf, stubs_total);
}

include!("../../patches_config.rs");
static mut REL_CONFIG: RelConfig = RelConfig {
    quickplay_mlvl: 0xFFFFFFFF,
    quickplay_mrea: 0xFFFFFFFF,
};

#[prolog_fn]
unsafe extern "C" fn setup_global_state() {
    {
        let mut fi = if let Some(fi) = DVDFileInfo::new(b"rel_config.bin\0") {
            fi
        } else {
            return;
        };
        let config_size = fi.file_length() as usize;
        let mut recv_buf = alloc::vec![MaybeUninit::<u8>::uninit(); config_size + 63];
        let recv_buf = Aligned32::split_unaligned_prefix_mut(&mut recv_buf[..]).1;
        let recv_buf = &mut recv_buf[..(config_size + 31) & !31];
        {
            let _ = fi.read_async(recv_buf, 0, 0);
        }
        REL_CONFIG = ssmarshal::deserialize(recv_buf[..config_size].assume_init())
            .unwrap()
            .0;
    }
}

#[patch_fn(kind = call,
           target = "FinishedLoading__19SNewFileSelectFrame" + 0x2c,
           version = Ntsc0_00)]
#[patch_fn(kind = call,
           target = "FinishedLoading__19SNewFileSelectFrame" + 0x2c,
           version = Ntsc0_01)]
#[patch_fn(kind = call,
           target = "FinishedLoading__19SNewFileSelectFrame" + 0x2c,
           version = Ntsc0_02)]
#[patch_fn(kind = call,
           target = "FinishedLoading__19SNewFileSelectFrame" + 0x2c,
           version = NtscK)]
#[patch_fn(kind = call,
           target = "FinishedLoading__19SNewFileSelectFrame" + 0x34,
           version = NtscJ)]
#[patch_fn(kind = call,
           target = "FinishedLoading__19SNewFileSelectFrame" + 0x34,
           version = Pal)]
#[allow(clippy::manual_c_str_literals)]
unsafe extern "C" fn update_main_menu_text(
    frame: *mut CGuiFrame,
    widget_name: *const u8,
) -> *mut CGuiWidget {
    let res = CGuiFrame::find_widget(frame, widget_name);

    let version = GameVersion::current();
    let str_idx = if version == GameVersion::Pal || version == GameVersion::NtscJ {
        104
    } else {
        110
    };
    let raw_string = CStringTable::get_string(CStringTable::main_string_table(), str_idx);
    let s = WString::from_ucs2_str(raw_string);

    for name in &[
        b"textpane_identifier\0".as_ptr(),
        b"textpane_identifierb\0".as_ptr(),
    ] {
        let widget = CGuiFrame::find_widget(frame, *name);
        let text_support = CGuiTextPane::text_support_mut(widget as *mut CGuiTextPane);
        CGuiTextSupport::set_text(text_support, &s);
    }

    res
}

// Based on
// https://github.com/AxioDL/PWEQuickplayPatch/blob/249ae82cc20031fe99894524aefb1f151430bedf/Source/QuickplayModule.cpp#L150
#[patch_fn(kind = call,
           target = "OnMessage__9CMainFlowFRC20CArchitectureMessageR18CArchitectureQueue" + 72)]
unsafe extern "C" fn quickplay_hook_advance_game_state(
    flow: *mut CMainFlow,
    q: *mut CArchitectureQueue,
) {
    static mut INIT: bool = false;
    if CMainFlow::game_state(flow) == CMainFlow::CLIENT_FLOW_STATE_PRE_FRONT_END && !INIT {
        INIT = true;
        if REL_CONFIG.quickplay_mlvl != 0xFFFFFFFF {
            let game_state = CGameState::global_instance();
            CGameState::set_current_world_id(game_state, REL_CONFIG.quickplay_mlvl);
            let world_state = CGameState::get_current_world_state(game_state);
            CWorldState::set_desired_area_asset_id(world_state, REL_CONFIG.quickplay_mrea);
            CMainFlow::set_game_state(flow, CMainFlow::CLIENT_FLOW_STATE_GAME, q);
            return;
        }
    }
    CMainFlow::advance_game_state(flow, q)
}
