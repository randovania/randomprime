use std::{env, fs, io::Write, path::Path, process::Command};

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

/// Write `bytes` to `path` only if its current contents differ. These outputs are
/// `include_bytes!`/`include_str!`'d by downstream crates, so rewriting them with identical
/// bytes would still bump their mtime and force an unnecessary recompile whenever this build
/// script re-runs (e.g. for an unrelated change in the watched ppc sources).
fn write_if_changed(path: &Path, bytes: &[u8]) {
    let unchanged = fs::read(path).map(|cur| cur == bytes).unwrap_or(false);
    if !unchanged {
        fs::write(path, bytes).unwrap();
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

    // Resolve sibling crates with clean (no `..`) absolute paths so Cargo records normalized
    // rerun-if-changed entries. `root_dir` is `<repo>/generated/rel_files`.
    let generated_dir = root_dir.parent().unwrap();
    let repo_dir = generated_dir.parent().unwrap();
    let ppc_dir = repo_dir.join("compile_to_ppc");
    let ppc_manifest = ppc_dir.join("Cargo.toml");
    let target_dir = ppc_dir
        .join("target")
        .join("powerpc-unknown-linux-gnu")
        .join("release");

    let symbol_table_dir = generated_dir.join("dol_symbol_table");

    invoke_cargo(&ppc_manifest, "rel_loader");
    invoke_cargo(&ppc_manifest, "rel_patches");

    let mut cave_base_addrs_bytes = Vec::new();
    for info in REL_LOADER_VERSION_INFO {
        writeln!(
            cave_base_addrs_bytes,
            "pub const {}: u32 = 0x{:08X};",
            info.cave_base_const_name, info.cave_base_addr
        )
        .unwrap();
    }
    write_if_changed(
        &out_dir.join("rel_loader_cave_base_addrs.rs"),
        &cave_base_addrs_bytes,
    );

    for info in REL_LOADER_VERSION_INFO {
        let version = info.version;
        let sym_table_path = symbol_table_dir.join(format!("{}.txt", version));
        let mut symbol_table = read_symbol_table(&sym_table_path).unwrap();

        let cave_bin_path = out_dir.join(format!("rel_loader_{}.cave.bin", version));
        let cave_bin_tmp = out_dir.join(format!("rel_loader_{}.cave.bin.tmp", version));
        let cave_symbols_map = link_obj_files_to_bin(
            [target_dir.join("librel_loader.a")].iter(),
            info.cave_base_addr,
            &symbol_table,
            &cave_bin_tmp,
        )
        .unwrap();
        write_if_changed(&cave_bin_path, &fs::read(&cave_bin_tmp).unwrap());
        fs::remove_file(&cave_bin_tmp).unwrap();

        // Sort so identical inputs always produce identical bytes (the linker returns the
        // symbols in a HashMap, whose iteration order is otherwise nondeterministic).
        let mut sorted_symbols = cave_symbols_map.clone();
        sorted_symbols.sort();
        let mut map_bytes = Vec::new();
        for (sym_name, addr) in &sorted_symbols {
            writeln!(map_bytes, "0x{:08x} {}", addr, sym_name).unwrap();
        }
        write_if_changed(&cave_bin_path.with_extension("bin.map"), &map_bytes);

        for (sym_name, addr) in cave_symbols_map {
            symbol_table.entry(sym_name).or_insert(addr);
        }

        let rel_path = out_dir.join(format!("patches_{}.rel", version));
        let rel_tmp = out_dir.join(format!("patches_{}.rel.tmp", version));
        link_obj_files_to_rel(
            [target_dir.join("librel_patches.a")].iter(),
            &symbol_table,
            &rel_tmp,
        )
        .unwrap();
        write_if_changed(&rel_path, &fs::read(&rel_tmp).unwrap());
        fs::remove_file(&rel_tmp).unwrap();
    }

    for watch_dir in [&ppc_dir, &symbol_table_dir] {
        let walkdir = WalkDir::new(watch_dir).into_iter().filter_entry(|entry| {
            let name = entry.file_name().to_str().unwrap_or("");
            !name.starts_with('.') && name != "target"
        });
        for entry in walkdir {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                println!("cargo:rerun-if-changed={}", entry.path().display());
            }
        }
    }
}
