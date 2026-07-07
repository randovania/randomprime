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

## Cross-version support

The six GameCube builds (NTSC-U 0-00/0-01/0-02, Korean, PAL, NTSC-J) have different symbol addresses and, for the localized builds, different struct layouts. Patches resolve addresses with `symbol_addr_opt!(<symbol>, version)` and define per-version symbol offsets for each unique patch site.

## Save-slot persistence

Randomprime stores its own data in each memory-card save slot. The game serializes `CGameState` into a fixed-size per-slot buffer (at least 940 bytes, depending on version). Randomprime stores its data in the **front** of the buffer: `CGameState`'s first serialized member is a hard-coded 128-byte array, written as the first 128 bytes of every save regardless of content. `PutTo` writes it and the load ctor reads it back; nothing else touches it, so it's safe to overwrite with our schema. The stamp hook writes into the live `CGameState` object at `PutTo` entry, and the object's own serialization loop writes those bytes to buffer offset 0 moments later.

The block layout is defined in `dol_patches.rs` (the `SAVE_SCHEMA_*` constants). Because the offsets are fixed, instances with different game configuration can still read each other's randomprime save data. All randomprime custom save blocks start with "RPSV" and a layout version number. Custom patches should early-exit if a save slot is missing this magic header or the version is unexpected.

## References

- [PowerPC 750CL ISA](https://fail0verflow.com/media/files/ppc_750cl.pdf) - Instruction encoding, condition registers, ABI
- [PowerPC ISA Appendix F](https://www.nxp.com/docs/en/user-guide/MPCFPE_AD_R1.pdf) - Simplified mnemonics
- [Compiler Explorer](https://godbolt.org/) - Compile C snippets to PPC asm to understand what the decompiler expects
