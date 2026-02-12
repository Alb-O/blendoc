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
     - `old u64` (stored address identifier)
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
   - Detects pointer storage mode:
     - `address_ranges` (legacy raw runtime pointers)
     - `stable_ids` (opaque stable identifiers written by modern Blender)
   - Resolves exact block-start identifiers in all modes.
   - Uses in-block offset/range fallback only for `address_ranges`.
   - Computes element-level position (`element_index`, `element_offset`) using SDNA sizes.

7. **Pointer chase primitives + path chase** (`src/blend/chase.rs`, `src/blend/path.rs`, `src/blend/chase_path.rs`)
   - One-step pointer chase into decoded struct instances.
   - Scene-specific convenience chase (`Scene.camera`, etc.).
   - Generic field path traversal with pointer auto-deref:
     - path grammar: `field.subfield[0].other`
   - Cycle/hop guards and stop policies (stop vs error behavior).

## CLI commands

All commands are under the `blendoc` binary:

- `blendoc info <file> [--json]`
  - header summary, pointer storage mode, pointer-ID diagnostics, block count, top block codes.
  - `--json` emits a machine-readable payload for fixture diff/comparison workflows.

- `blendoc dna <file> [--struct <Name>]`
  - SDNA table counts and optional struct field dump.

- `blendoc decode <file> --code <CODE>`
  - decode first block by code into typed values.
  - output has truncation controls for arrays/strings/nesting.

- `blendoc scene <file>`
  - convenience decode for first `SC\0\0` block using scene-focused print/decode defaults.

- `blendoc camera <file>`
  - one-step chase from scene camera pointer to target object (if non-null/resolvable).

- `blendoc ids <file> [--code <CODE>] [--type <StructName>] [--limit <N>] [--json]`
  - scan ID-root blocks and print `ID.name` plus useful ID header pointers.
  - optional filtering by block code or derived struct type.

- `blendoc chase <file> (--code <CODE> | --ptr <HEX> | --id <IDNAME>) --path <FIELD.PATH> [--json]`
  - run generic field-path chase with hop-by-hop pointer trace.
  - hop output includes resolved type metadata and ID-name annotation when available.

- `blendoc refs <file> (--code <CODE> | --ptr <HEX> | --id <IDNAME>) [--depth <N>] [--limit <N>] [--json]`
  - scan pointer-valued fields from one root struct and attempt pointer resolution.
  - includes canonical target metadata and ID-name annotations when available.

- `blendoc graph <file> (--code <CODE> | --ptr <HEX> | --id <IDNAME>) [--depth <N>] [--refs-depth <N>] [--max-nodes <N>] [--max-edges <N>] [--id-only] [--dot] [--json]`
  - build a shallow pointer graph from one root pointer with BFS limits.
  - supports text, Graphviz DOT, and JSON output formats.

- `blendoc xref <file> (--id <IDNAME> | --ptr <HEX>) [--refs-depth <N>] [--limit <N>] [--json]`
  - find inbound references to a target canonical pointer.
  - reports owner ID/type and pointer field path for each inbound edge.

- `blendoc route <file> (--from-id <NAME> | --from-ptr <HEX> | --from-code <CODE>) (--to-id <NAME> | --to-ptr <HEX>) [--depth <N>] [--refs-depth <N>] [--max-nodes <N>] [--max-edges <N>] [--json]`
  - find a shortest pointer route between canonicalized endpoints.
  - reports traversal budgets, truncation reason, and route edges when found.

- `blendoc idgraph <file> [--refs-depth <N>] [--max-edges <N>] [--dot] [--json] [--prefix <XX>] [--type <Name>]`
  - build a whole-file ID-to-ID graph across ID-root records.
  - supports optional node filtering by ID name prefix or type.

- `blendoc show <file> (--id <IDNAME> | --ptr <HEX> | --code <CODE>) [--path <FIELD.PATH>] [--trace] [--json] [--max-depth <N>] [--max-array <N>] [--include-padding] [--strict-layout] [--annotate-ptrs|--raw-ptrs] [--expand-depth <N>] [--expand-max-nodes <N>]`
  - decode and print a struct instance from a pointer-like selector.
  - optional `--path` mode evaluates a chased field path from the selected root.
  - pointer fields can be annotated inline with resolved type/ID metadata.

- `blendoc walk <file> (--id <IDNAME> | --ptr <HEX> | --code <CODE>) [--path <FIELD.PATH>] [--next <FIELD>] [--refs-depth <N>] [--limit <N>] [--json]`
  - walk linked pointer chains by repeatedly following one pointer field.
  - supports path-derived walk starts and structured stop reasons.

Examples:

```bash
nix develop -c cargo run -- info fixtures/character.blend
nix develop -c cargo run -- info fixtures/v5.1_character.blend --json
nix develop -c cargo run -- dna fixtures/character.blend --struct Scene
nix develop -c cargo run -- decode fixtures/character.blend --code GLOB
nix develop -c cargo run -- scene fixtures/character.blend
nix develop -c cargo run -- camera fixtures/character.blend
nix develop -c cargo run -- chase fixtures/character.blend --code SC --path world
nix develop -c cargo run -- refs fixtures/character.blend --id SCScene --depth 1
nix develop -c cargo run -- graph fixtures/character.blend --id SCScene --depth 1 --refs-depth 1
nix develop -c cargo run -- xref fixtures/character.blend --id WOWorld --limit 10
nix develop -c cargo run -- route fixtures/character.blend --from-id SCScene --to-id WOWorld --depth 3 --refs-depth 1
nix develop -c cargo run -- idgraph fixtures/character.blend --refs-depth 1 --max-edges 200
nix develop -c cargo run -- show fixtures/character.blend --id WOWorld
nix develop -c cargo run -- show fixtures/character.blend --id WOWorld --expand-depth 1
nix develop -c cargo run -- walk fixtures/character.blend --id SCScene --next id.next --limit 20
```

Example fixture comparison snippet:

```bash
nix develop -c cargo run -- info fixtures/v5.1_character.blend --json > /tmp/char.json
nix develop -c cargo run -- info fixtures/v5.1_sword.blend --json > /tmp/sword.json
jq '{path, version, pointer_storage, pointer_diagnostics}' /tmp/char.json /tmp/sword.json
```

## Library entry points

Main types are re-exported through `blendoc::blend` (`src/blend/mod.rs`).

Core entry points:

- `BlendFile::open(path)`
- `BlendFile::blocks()`
- `BlendFile::dna()`
- `BlendFile::pointer_index()`
- `decode_block_instances(...)`
- `decode_ptr_instance(...)`
- `chase_ptr_to_struct(...)`
- `chase_from_block_code(...)`, `chase_from_ptr(...)`
- `FieldPath::parse(...)`
- `scan_id_blocks(...)`
- `scan_refs_from_ptr(...)`
- `build_graph_from_ptr(...)`
- `find_inbound_refs_to_ptr(...)`
- `find_route_between_ptrs(...)`
- `build_id_graph(...)`
- `walk_ptr_chain(...)`

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
