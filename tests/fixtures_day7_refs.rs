#![allow(missing_docs)]

use std::path::{Path, PathBuf};

use blendoc::blend::{BlendFile, IdIndex, RefScanOptions, scan_id_blocks, scan_refs_from_ptr};

#[test]
fn character_scene_refs_include_world() {
	assert_scene_world_ref("character.blend");
}

#[test]
fn sword_scene_refs_include_world() {
	assert_scene_world_ref("sword.blend");
}

fn assert_scene_world_ref(name: &str) {
	let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
	let dna = blend.dna().expect("dna parses");
	let index = blend.pointer_index().expect("pointer index builds");
	let ids = scan_id_blocks(&blend, &dna).expect("id scan succeeds");
	let id_index = IdIndex::build(ids.clone());

	let scene = ids.iter().find(|item| item.code == [b'S', b'C', 0, 0]).expect("scene id record exists");

	let refs = scan_refs_from_ptr(
		&dna,
		&index,
		&id_index,
		scene.old_ptr,
		&RefScanOptions {
			max_depth: 1,
			max_array_elems: 4096,
		},
	)
	.expect("ref scan succeeds");

	let world = refs.iter().find(|item| item.field.as_ref() == "world").expect("world ref exists");
	let target = world.resolved.as_ref().expect("world should resolve");
	assert_eq!(target.type_name.as_ref(), "World");

	let id_name = target.id_name.as_deref().expect("world id annotation exists");
	assert!(id_name.starts_with("WO"), "expected World ID prefix");
}

fn fixture_path(name: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}
