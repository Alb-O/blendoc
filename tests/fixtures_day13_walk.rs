#![allow(missing_docs)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use blendoc::blend::{BlendFile, IdIndex, RefScanOptions, StopMode, WalkOptions, scan_id_blocks, walk_ptr_chain};

#[test]
fn character_scene_id_next_walk_smoke() {
	assert_scene_walk("character.blend");
}

#[test]
fn sword_scene_id_next_walk_smoke() {
	assert_scene_walk("sword.blend");
}

fn assert_scene_walk(name: &str) {
	let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
	let dna = blend.dna().expect("dna parses");
	let index = blend.pointer_index().expect("pointer index builds");
	let ids = IdIndex::build(scan_id_blocks(&blend, &dna).expect("id scan succeeds"));

	let scene = ids.get_by_name("SCScene").expect("scene id exists");
	let result = walk_ptr_chain(
		&dna,
		&index,
		&ids,
		scene.old_ptr,
		&WalkOptions {
			next_field: Arc::<str>::from("id.next"),
			max_steps: 64,
			ref_scan: RefScanOptions {
				max_depth: 1,
				max_array_elems: 4096,
			},
			on_null: StopMode::Stop,
			on_unresolved: StopMode::Stop,
			on_cycle: StopMode::Stop,
		},
	)
	.expect("walk succeeds");

	assert!(!result.items.is_empty(), "expected at least one walked item");
	let first = &result.items[0];
	let id_name = first.id_name.as_deref().expect("first scene item should have id name");
	assert!(id_name.starts_with("SC"), "expected Scene ID prefix");
}

fn fixture_path(name: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}
