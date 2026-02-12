#![allow(missing_docs)]

use std::path::{Path, PathBuf};

use blendoc::blend::{BlendFile, IdIndex, RefScanOptions, RouteOptions, find_route_between_ptrs, scan_id_blocks};

#[test]
fn character_scene_to_world_route_is_direct() {
	assert_scene_world_route("character.blend");
}

#[test]
fn sword_scene_to_world_route_is_direct() {
	assert_scene_world_route("sword.blend");
}

fn assert_scene_world_route(name: &str) {
	let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
	let dna = blend.dna().expect("dna parses");
	let index = blend.pointer_index().expect("pointer index builds");
	let ids = IdIndex::build(scan_id_blocks(&blend, &dna).expect("id scan succeeds"));

	let scene = ids.get_by_name("SCScene").expect("scene id record exists");
	let world = ids.get_by_name("WOWorld").expect("world id record exists");

	let result = find_route_between_ptrs(
		&dna,
		&index,
		&ids,
		scene.old_ptr,
		world.old_ptr,
		&RouteOptions {
			max_depth: 3,
			max_nodes: 4096,
			max_edges: 16384,
			ref_scan: RefScanOptions {
				max_depth: 1,
				max_array_elems: 4096,
			},
		},
	)
	.expect("route search succeeds");

	let path = result.path.expect("expected route to be found");
	assert_eq!(path.len(), 1);
	assert_eq!(path[0].field.as_ref(), "world");
	assert_eq!(path[0].from, scene.old_ptr);
	assert_eq!(path[0].to, world.old_ptr);
}

fn fixture_path(name: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}
