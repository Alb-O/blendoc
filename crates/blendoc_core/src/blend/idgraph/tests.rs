mod fixtures_day11_idgraph {

	use blendoc_testkit::fixture_path;

	use crate::blend::{BlendFile, IdGraphOptions, IdIndex, RefScanOptions, build_id_graph, scan_id_blocks};

	#[test]
	fn character_idgraph_has_scene_world_edge() {
		assert_idgraph_world_edge("character.blend");
	}

	#[test]
	fn sword_idgraph_has_scene_world_edge() {
		assert_idgraph_world_edge("sword.blend");
	}

	fn assert_idgraph_world_edge(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let index = blend.pointer_index().expect("pointer index builds");
		let ids = IdIndex::build(scan_id_blocks(&blend, &dna).expect("id scan succeeds"));

		let scene = ids.get_by_name("SCScene").expect("scene id exists");
		let world = ids.get_by_name("WOWorld").expect("world id exists");

		let graph = build_id_graph(
			&dna,
			&index,
			&ids,
			&IdGraphOptions {
				ref_scan: RefScanOptions {
					max_depth: 1,
					max_array_elems: 4096,
				},
				max_edges: 100_000,
				include_self: false,
			},
		)
		.expect("id graph builds");

		assert!(
			graph
				.edges
				.iter()
				.any(|edge| edge.from == scene.old_ptr && edge.to == world.old_ptr && edge.field.as_ref() == "world"),
			"expected SCScene -world-> WOWorld edge"
		);
	}
}
