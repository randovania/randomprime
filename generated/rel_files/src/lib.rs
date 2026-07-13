include!(concat!(env!("OUT_DIR"), "/rel_loader_cave_base_addrs.rs"));

pub const REL_LOADER_100_CAVE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_1.00.cave.bin"));
pub const REL_LOADER_100_CAVE_MAP: &str =
    include_str!(concat!(env!("OUT_DIR"), "/rel_loader_1.00.cave.bin.map"));
pub const REL_LOADER_101_CAVE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_1.01.cave.bin"));
pub const REL_LOADER_101_CAVE_MAP: &str =
    include_str!(concat!(env!("OUT_DIR"), "/rel_loader_1.01.cave.bin.map"));
pub const REL_LOADER_102_CAVE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_1.02.cave.bin"));
pub const REL_LOADER_102_CAVE_MAP: &str =
    include_str!(concat!(env!("OUT_DIR"), "/rel_loader_1.02.cave.bin.map"));
pub const REL_LOADER_PAL_CAVE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_pal.cave.bin"));
pub const REL_LOADER_PAL_CAVE_MAP: &str =
    include_str!(concat!(env!("OUT_DIR"), "/rel_loader_pal.cave.bin.map"));
pub const REL_LOADER_KOR_CAVE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_kor.cave.bin"));
pub const REL_LOADER_KOR_CAVE_MAP: &str =
    include_str!(concat!(env!("OUT_DIR"), "/rel_loader_kor.cave.bin.map"));
pub const REL_LOADER_JPN_CAVE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_jpn.cave.bin"));
pub const REL_LOADER_JPN_CAVE_MAP: &str =
    include_str!(concat!(env!("OUT_DIR"), "/rel_loader_jpn.cave.bin.map"));
pub const PATCHES_100_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_1.00.rel"));
pub const PATCHES_101_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_1.01.rel"));
pub const PATCHES_102_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_1.02.rel"));
pub const PATCHES_PAL_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_pal.rel"));
pub const PATCHES_KOR_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_kor.rel"));
pub const PATCHES_JPN_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_jpn.rel"));
