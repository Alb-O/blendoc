# blendoc

Rust tooling for reverse engineering Blender `.blend` files, scoped to Blender 5.0+.

This project is intentionally format-focused. It implements parsing, schema decoding (SDNA), pointer resolution, and controlled pointer/path traversal over real fixture files.

## Scope and assumptions

- Target format: Blender file-format v1 headers (`BLENDER17-01v0500` and newer).
- Blender version gate: `>= 5.0` only.
- Endianness: little-endian only.
- Block header layout: `LargeBHead8` only.
- Input compression: uncompressed or zstd-compressed streams.
- Fixtures currently tracked: `fixtures/character.blend`, `fixtures/sword.blend`.

Out of scope right now:

- Legacy pre-v1 `.blend` headers.
- Big-endian handling.
- Full recursive graph extraction / full scene reconstruction.

## Format pipeline implemented

The implementation is split into explicit layers:

1. **Input + compression** (`src/blend/compression.rs`)
   - Detects `BLENDER` magic (raw) or zstd frame magic (`28 B5 2F FD`).
   - Decompresses zstd with an explicit output cap.
   - Verifies decompressed stream begins with `BLENDER`.

2. **Header parse** (`src/blend/header.rs`)
   - Parses v1 header fields:
     - header size
     - format version
     - blender version
   - Enforces v1 and Blender `>= 500`.
   - Enforces little-endian marker.

3. **Block iteration** (`src/blend/bhead.rs`, `src/blend/block.rs`)
   - Parses `LargeBHead8` as:
     - `code [u8;4]`
     - `sdna_nr u32`
     - `old u64`
     - `len i64` (validated non-negative)
     - `nr i64` (validated non-negative)
   - Yields safe `Block` views with payload slices and file offsets.

4. **DNA/SDNA parse** (`src/blend/dna.rs`)
   - Parses `SDNA` sections: `NAME`, `TYPE`, `TLEN`, `STRC`.
   - Handles 4-byte alignment boundaries.
   - Validates type/name indices.
   - Builds fast type->struct lookup (`struct_for_type`).

5. **Typed decode** (`src/blend/decode.rs`, `src/blend/value.rs`)
   - Decodes struct instances from SDNA metadata.
   - Supports field declarators:
     - pointers (`*field`)
     - parenthesized pointers (`(*field)`, `(**field)`)
     - inline arrays (`field[n]`)
     - zero-length arrays (`field[0]` preserved)
   - Decode guardrails:
     - max depth
     - max array elements
     - payload size precheck
     - optional strict layout check

6. **Pointer indexing and typed resolution** (`src/blend/pointer.rs`)
   - Builds range index from `old` pointer bases to payload spans.
   - Resolves pointers both to exact block starts and in-block offsets.
   - Computes element-level position (`element_index`, `element_offset`) using SDNA sizes.

7. **Pointer chase primitives + path chase** (`src/blend/chase.rs`, `src/blend/path.rs`, `src/blend/chase_path.rs`)
   - One-step pointer chase into decoded struct instances.
   - Scene-specific convenience chase (`Scene.camera`, etc.).
   - Generic field path traversal with pointer auto-deref:
     - path grammar: `field.subfield[0].other`
   - Cycle/hop guards and stop policies (stop vs error behavior).

## CLI commands

All commands are under the `blendoc` binary:

- `blendoc info <file>`
  - header summary, compression, block count, top block codes.

- `blendoc dna <file> [--struct <Name>]`
  - SDNA table counts and optional struct field dump.

- `blendoc decode <file> --code <CODE>`
  - decode first block by code into typed values.
  - output has truncation controls for arrays/strings/nesting.

- `blendoc scene <file>`
  - convenience decode for first `SC\0\0` block using scene-focused print/decode defaults.

- `blendoc camera <file>`
  - one-step chase from scene camera pointer to target object (if non-null/resolvable).

Examples:

```bash
nix develop -c cargo run -- info fixtures/character.blend
nix develop -c cargo run -- dna fixtures/character.blend --struct Scene
nix develop -c cargo run -- decode fixtures/character.blend --code GLOB
nix develop -c cargo run -- scene fixtures/character.blend
nix develop -c cargo run -- camera fixtures/character.blend
```

## Library entry points

Main types are re-exported through `blendoc::blend` (`src/blend/mod.rs`).

Core entry points:

- `BlendFile::open(path)`
- `BlendFile::blocks()`
- `BlendFile::dna()`
- `BlendFile::pointer_index()`
- `decode_block_instances(...)`
- `chase_ptr_to_struct(...)`
- `chase_from_block_code(...)`, `chase_from_ptr(...)`
- `FieldPath::parse(...)`

Minimal usage sketch:

```rust
use blendoc::blend::{
    BlendFile, DecodeOptions, FieldPath, ChasePolicy, chase_from_block_code,
};

let file = BlendFile::open("fixtures/character.blend")?;
let dna = file.dna()?;
let index = file.pointer_index()?;

let mut decode = DecodeOptions::for_scene_inspect();
decode.include_padding = true;
decode.strict_layout = true;

let path = FieldPath::parse("world")?;
let result = chase_from_block_code(
    &file,
    &dna,
    &index,
    [b'S', b'C', 0, 0],
    &path,
    &decode,
    &ChasePolicy::default(),
)?;
```

## Development environment

Nix flake is the expected workflow.

- Enter shell: `nix develop`
- Run commands in shell context: `nix develop -c <cmd>`
- Formatter setup via treefmt in flake.
- Rust toolchain pinned in `rust-toolchain.toml`.

Typical loop:

```bash
nix develop -c cargo fmt
nix develop -c cargo test
```

## Test coverage (current)

Fixture and unit tests cover:

- header/block/decompression smoke checks (`tests/fixtures_day1.rs`)
- SDNA parse sanity (`tests/fixtures_day2_dna.rs`)
- typed decode sanity + strict-layout checks (`tests/fixtures_day3_decode.rs`, `tests/fixtures_day3_scene.rs`)
- pointer index and typed pointer resolution (`tests/fixtures_day4_pointers.rs`)
- one-step chase behavior (camera/world) (`tests/fixtures_day4_chase_camera.rs`)
- generic field-path chase (`tests/fixtures_day4_chase_path.rs`)
- synthetic cycle guard behavior (`tests/unit_chase_cycle.rs`)

## Known behavior in current fixtures

- `Scene.camera` is null in both shipped fixtures.
- `Scene.world` resolves and is used for positive pointer-chase assertions.

This is expected fixture data, not a parser failure.
