use std::{env, fs::File, io::Write, path::Path, process::Command};

use dol_linker::{link_obj_files_to_bin, link_obj_files_to_rel, read_symbol_table};
use walkdir::WalkDir;

fn invoke_cargo(ppc_manifest: &Path, package: &str) {
    let output = Command::new("cargo")
        .arg("rustc")
        .arg("--manifest-path")
        .arg(ppc_manifest)
        .arg("-p")
        .arg(package)
        .arg("--target")
        .arg("powerpc-unknown-linux-gnu")
        .arg("--release")
        .arg("--")
        .arg("-C")
        .arg("relocation-model=static")
        .arg("-C")
        .arg("target-cpu=750")
        .env("CARGO_TARGET_DIR", "../../compile_to_ppc/target")
        .output()
        .expect("Failed to compile ppc crate");
    if !output.status.success() {
        panic!("{}", String::from_utf8_lossy(&output.stderr));
    }
}

struct RelLoaderVersionInfo {
    version: &'static str,
    cave_base_addr: u32,
    cave_base_const_name: &'static str,
}

const REL_LOADER_VERSION_INFO: &[RelLoaderVersionInfo] = &[
    RelLoaderVersionInfo {
        version: "1.00",
        cave_base_addr: 0x8000C9BC,
        cave_base_const_name: "REL_LOADER_100_CAVE_BASE",
    },
    RelLoaderVersionInfo {
        version: "1.01",
        cave_base_addr: 0x8000CA38,
        cave_base_const_name: "REL_LOADER_101_CAVE_BASE",
    },
    RelLoaderVersionInfo {
        version: "1.02",
        cave_base_addr: 0x8000CC78,
        cave_base_const_name: "REL_LOADER_102_CAVE_BASE",
    },
    RelLoaderVersionInfo {
        version: "pal",
        cave_base_addr: 0x8000CF34,
        cave_base_const_name: "REL_LOADER_PAL_CAVE_BASE",
    },
    RelLoaderVersionInfo {
        version: "kor",
        cave_base_addr: 0x8000CA30,
        cave_base_const_name: "REL_LOADER_KOR_CAVE_BASE",
    },
    RelLoaderVersionInfo {
        version: "jpn",
        cave_base_addr: 0x8000D224,
        cave_base_const_name: "REL_LOADER_JPN_CAVE_BASE",
    },
];

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let root_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = Path::new(&root_dir);

    let ppc_dir = root_dir.join("..").join("..").join("compile_to_ppc");
    let ppc_manifest = ppc_dir.join("Cargo.toml");
    let target_dir = ppc_dir
        .join("..")
        .join("compile_to_ppc")
        .join("target")
        .join("powerpc-unknown-linux-gnu")
        .join("release");

    let symbol_table_dir = root_dir.join("..").join("dol_symbol_table");

    invoke_cargo(&ppc_manifest, "rel_loader");
    invoke_cargo(&ppc_manifest, "rel_patches");

    let cave_base_addrs_rs_path = out_dir.join("rel_loader_cave_base_addrs.rs");
    let mut cave_base_addrs_rs = File::create(cave_base_addrs_rs_path).unwrap();
    for info in REL_LOADER_VERSION_INFO {
        writeln!(
            cave_base_addrs_rs,
            "pub const {}: u32 = 0x{:08X};",
            info.cave_base_const_name, info.cave_base_addr
        )
        .unwrap();
    }

    for info in REL_LOADER_VERSION_INFO {
        let version = info.version;
        let sym_table_path = symbol_table_dir.join(format!("{}.txt", version));
        eprintln!("{:?}", root_dir.join("..").join(&sym_table_path));
        let mut symbol_table = read_symbol_table(root_dir.join(sym_table_path)).unwrap();

        let cave_bin_path = out_dir.join(format!("rel_loader_{}.cave.bin", version));
        let cave_symbols_map = link_obj_files_to_bin(
            [target_dir.join("librel_loader.a")].iter(),
            info.cave_base_addr,
            &symbol_table,
            &cave_bin_path,
        )
        .unwrap();
        {
            let cave_map_path = cave_bin_path.with_extension("bin.map");
            let mut cave_map_file = File::create(cave_map_path).unwrap();
            for (sym_name, addr) in &cave_symbols_map {
                writeln!(cave_map_file, "0x{:08x} {}", addr, sym_name).unwrap();
            }
        }

        for (sym_name, addr) in cave_symbols_map {
            symbol_table.entry(sym_name).or_insert(addr);
        }

        let rel_path = out_dir.join(format!("patches_{}.rel", version));
        link_obj_files_to_rel(
            [target_dir.join("librel_patches.a")].iter(),
            &symbol_table,
            &rel_path,
        )
        .unwrap();
    }

    for watch_dir in [&ppc_dir, &symbol_table_dir] {
        let walkdir = WalkDir::new(watch_dir).into_iter().filter_entry(|entry| {
            let name = entry.file_name().to_str().unwrap_or("");
            !name.starts_with('.') && name != "target"
        });
        for entry in walkdir {
            let entry = entry.unwrap();
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }
}
