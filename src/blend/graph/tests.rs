mod fixtures_day8_graph {

	use std::path::{Path, PathBuf};

	use crate::blend::{BlendFile, GraphOptions, RefScanOptions, build_graph_from_ptr, scan_id_blocks};

	#[test]
	fn character_scene_graph_has_world_edge() {
		assert_scene_graph("character.blend");
	}

	#[test]
	fn sword_scene_graph_has_world_edge() {
		assert_scene_graph("sword.blend");
	}

	fn assert_scene_graph(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let index = blend.pointer_index().expect("pointer index builds");
		let ids = crate::blend::IdIndex::build(scan_id_blocks(&blend, &dna).expect("id scan succeeds"));

		let scene = ids.get_by_name("SCScene").expect("SCScene id exists");

		let graph = build_graph_from_ptr(
			&dna,
			&index,
			&ids,
			scene.old_ptr,
			&GraphOptions {
				max_depth: 1,
				max_nodes: 4096,
				max_edges: 16384,
				ref_scan: RefScanOptions {
					max_depth: 1,
					max_array_elems: 4096,
				},
				id_only: false,
				skip_null_ptrs: true,
			},
		)
		.expect("graph builds");

		let world = graph.nodes.iter().find(|node| node.type_name.as_ref() == "World").expect("world node exists");
		let world_id = world.id_name.as_deref().expect("world id annotation exists");
		assert!(world_id.starts_with("WO"), "expected World ID prefix");

		assert!(
			graph
				.edges
				.iter()
				.any(|edge| edge.from == scene.old_ptr && edge.to == world.canonical && edge.field.as_ref() == "world"),
			"expected Scene.world edge"
		);
	}

	fn fixture_path(name: &str) -> PathBuf {
		Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
	}
}
