#![allow(missing_docs)]

use std::path::{Path, PathBuf};

use blendoc::blend::{BlendFile, IdIndex, RefScanOptions, XrefOptions, find_inbound_refs_to_ptr, scan_id_blocks};

#[test]
fn character_world_xref_includes_scene_world_field() {
	assert_world_xref("character.blend");
}

#[test]
fn sword_world_xref_includes_scene_world_field() {
	assert_world_xref("sword.blend");
}

fn assert_world_xref(name: &str) {
	let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
	let dna = blend.dna().expect("dna parses");
	let index = blend.pointer_index().expect("pointer index builds");
	let ids = IdIndex::build(scan_id_blocks(&blend, &dna).expect("id scan succeeds"));

	let world = ids
		.records
		.iter()
		.find(|item| item.type_name.as_ref() == "World" || item.code == [b'W', b'O', 0, 0])
		.expect("world id record exists");

	let inbound = find_inbound_refs_to_ptr(
		&dna,
		&index,
		&ids,
		world.old_ptr,
		&XrefOptions {
			ref_scan: RefScanOptions {
				max_depth: 1,
				max_array_elems: 4096,
			},
			max_results: 512,
			include_unresolved: false,
		},
	)
	.expect("xref query succeeds");

	assert!(
		inbound
			.iter()
			.any(|item| item.field.as_ref() == "world" && item.from_id.as_deref().is_some_and(|id| id.starts_with("SC"))),
		"expected Scene.world inbound edge"
	);
}

fn fixture_path(name: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}
