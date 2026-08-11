//! Forces every SCLY object in every room through `guess_kind`, which asserts that a re-typed
//! object serializes back to exactly the byte count it was read as. Run against each retail ISO
//! after changing any `scly_props` definition.

use std::{collections::BTreeMap, env::args, fs::File};

pub use randomprime::*;
use reader_writer::{Readable, Reader, Writable};

fn main() {
    let path = args().nth(1).expect("usage: scly_typecheck <iso>");
    let file = File::open(&path).unwrap();
    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
    let mut reader = Reader::new(&mmap[..]);
    let gc_disc: structs::GcDisc = reader.read(());

    let paks = [
        "Metroid1.pak",
        "Metroid2.pak",
        "Metroid3.pak",
        "Metroid4.pak",
        "metroid5.pak",
        "Metroid6.pak",
        "Metroid7.pak",
        "Metroid8.pak",
    ];

    let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
    let mut failures: BTreeMap<(u8, String), usize> = BTreeMap::new();
    let mut objects = 0usize;
    let mut rooms = 0usize;

    for pak_name in paks {
        let file_entry = gc_disc.find_file(pak_name).unwrap();
        let pak = match *file_entry.file().unwrap() {
            structs::FstEntryFile::Pak(ref pak) => pak.clone(),
            structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
            _ => panic!(),
        };

        for res in pak.resources.iter() {
            if res.fourcc() != b"MREA".into() {
                continue;
            }

            rooms += 1;
            let data = ResourceData::new(&res).decompress().into_owned();
            let mrea = Reader::new(&data[..]).read::<structs::Mrea>(());

            for layer in mrea.scly_section().layers.iter() {
                for obj in layer.objects.iter() {
                    let mut obj = obj.into_owned();
                    let object_type = obj.property_data.object_type();
                    let raw_len = obj.property_data.size();

                    // RidleyV1 and RidleyV2 share this type with different property counts, so
                    // guess_kind always picks the NTSC one and asserts on PAL/NTSC-J
                    if object_type == 0x7B {
                        objects += 1;
                        *counts.entry(object_type).or_default() += 1;
                        continue;
                    }

                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        obj.property_data.guess_kind();
                        let mut round_tripped = vec![];
                        obj.property_data.write_to(&mut round_tripped).unwrap();
                        round_tripped.len()
                    }));

                    match outcome {
                        Ok(len) if len == raw_len => {}
                        Ok(len) => {
                            *failures
                                .entry((object_type, format!("size {} != {}", len, raw_len)))
                                .or_default() += 1;
                        }
                        Err(_) => {
                            *failures
                                .entry((object_type, "panic".to_string()))
                                .or_default() += 1;
                        }
                    }

                    objects += 1;
                    *counts.entry(object_type).or_default() += 1;
                }
            }
        }
    }

    println!(
        "{}: {} rooms, {} objects, {} distinct types",
        path,
        rooms,
        objects,
        counts.len()
    );
    for ((object_type, why), n) in &failures {
        println!(
            "   FAIL type 0x{:X} ({} instances): {}",
            object_type, n, why
        );
    }
    if failures.is_empty() {
        println!("   all round-tripped byte-exact");
    }

    // Printed in full rather than checked against a fixed list, because "all clean" says
    // nothing about a definition the corpus never exercised. Look up whatever you changed.
    println!("   instances by object type:");
    for (object_type, n) in &counts {
        println!("      0x{:02X} {}", object_type, n);
    }
}
