# DOL Patching

The `.dol` is the game's (PowerPC) executable. Randomprime modifies it when creating a new ISO to edit or extend game behavior. Patches are defined in `dol_patches.rs`.

## Patch strategies

### Direct replacement

The simplest change is writing new instructions directly over existing ones with `dol_patcher.ppcasm_patch` such as "nop"-ing a vanilla instruction to negate its effect. `ppcasm!` is used assemble the instructions from inline mnemonics.

### Code cave (hook + trampoline)

When a patch needs to add behavior instead of replacing:

1. Pick a hook site: Find an instruction that runs at the right time
2. `emitter.emit_and_patch` overwrites that site with a `bl` into a **code cave** (unused space in the DOL)
3. Execute the original displaced instruction, then new functionality, finally jump back to `hook_site + 4`

Data whose address needs to be referenced is reserved early via `emit_addressed` (e.g. `reserve_save_uuid_data`). That data has no overflow path, so it must be reserved before any  code caves are written to.

### Code cave overflow

Caves are finite (defined per-version in `caves_for_version`). When full, `emit_and_patch` overflows into `cave_overflow.bin` (transparent to the caller). At game boot the REL loader (`apply_cave_overflow`) allocates the stub on the heap, applies the relocations, and writes the `bl` into the hook site.

#### `cave_overflow.bin` format

Written by `serialize_overflow` and read by `apply_cave_overflow`. All fields are big-endian `u32`. Because a stub runs from a heap address unknown at patch time, it is made position-independent (`make_pic`) and carries relocations the loader fixes up once the heap address is known. An empty file (or a stub/total count of zero) means there was no overflow, and the loader returns without patching anything.

```
Header (8 bytes)
  u32  stub_count      number of stub records that follow
  u32  stubs_total     sum of every record's stub_size; the loader mallocs one
                       heap buffer of this size and copies all stubs into it

Repeated stub_count times:
  u32  dol_patch_site  DOL address of the hook instruction to overwrite with a
                       branch to the stub's heap address
  u32  patch_is_bl     branch kind written at the hook site: 1 = bl (link), 0 = b
  u32  stub_size       length in bytes of the stub code that follows
  u8   stub_bytes[stub_size]   position-independent stub machine code

  u32  reloc_count     number of relocation records for this stub
  Repeated reloc_count times (16 bytes each):
    u32  lis_pos       byte offset within the stub of the `lis` instruction
    u32  lo_pos        byte offset within the stub of the paired low-half
                       instruction (`addi` or `ori`)
    u32  data_off      byte offset within the stub of the data word whose final
                       heap address the pair must materialize
    u32  is_addi       low-half instruction form: 1 = addi (sign-extended low
                       half, so the loader biases the high half when bit 15 is
                       set), 0 = ori (zero-extended)
```

Each relocation patches the immediate fields of the `lis`/`addi|ori` pair so the stub loads `heap_addr + data_off` at runtime instead of the dummy `STUB_COMPILE_BASE` address it was assembled against.

## Cross-version support

The six GameCube builds (NTSC-U 0-00/0-01/0-02, Korean, PAL, NTSC-J) have different symbol addresses and, for the localized builds, different struct layouts. Patches resolve addresses with `symbol_addr_opt!(<symbol>, version)` and define per-version symbol offsets for each unique patch site.

## Save-slot persistence

Randomprime stores its own data in each memory-card save slot. The game serializes `CGameState` into a fixed-size per-slot buffer (at least 940 bytes, depending on version). Randomprime stores its data in the **front** of the buffer: `CGameState`'s first serialized member is a hard-coded 128-byte array, written as the first 128 bytes of every save regardless of content. `PutTo` writes it and the load ctor reads it back; nothing else touches it, so it's safe to overwrite with our schema. The stamp hook writes into the live `CGameState` object at `PutTo` entry, and the object's own serialization loop writes those bytes to buffer offset 0 moments later.

The block layout is defined in `dol_patches.rs` (the `SAVE_SCHEMA_*` constants). Because the offsets are fixed, instances with different game configuration can still read each other's randomprime save data. All randomprime custom save blocks start with "RPSV" and a layout version number. Custom patches should early-exit if a save slot is missing this magic header or the version is unexpected.

## References

- [PowerPC 750CL ISA](https://fail0verflow.com/media/files/ppc_750cl.pdf) - Instruction encoding, condition registers, ABI
- [PowerPC ISA Appendix F](https://www.nxp.com/docs/en/user-guide/MPCFPE_AD_R1.pdf) - Simplified mnemonics
- [Compiler Explorer](https://godbolt.org/) - Compile C snippets to PPC asm to understand what the decompiler expects
