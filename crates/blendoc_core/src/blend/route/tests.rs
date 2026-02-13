mod fixtures_day10_route {

	use blendoc_testkit::fixture_path;

	use crate::blend::{BlendFile, IdIndex, RefScanOptions, RouteOptions, find_route_between_ptrs, scan_id_blocks};

	#[test]
	fn character_scene_to_world_route_is_direct() {
		assert_scene_world_route("character.blend");
	}

	#[test]
	fn sword_scene_to_world_route_is_direct() {
		assert_scene_world_route("sword.blend");
	}

	#[test]
	fn character_v51_scene_to_world_route_is_direct() {
		assert_scene_world_route("v5.1_character.blend");
	}

	#[test]
	fn sword_v51_scene_to_world_route_is_direct() {
		assert_scene_world_route("v5.1_sword.blend");
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
}

mod synthetic_chain {
	use crate::blend::{
		BHead, Block, Dna, DnaField, DnaStruct, IdIndex, IdRecord, PointerIndex, PtrEntry, RefScanOptions, RouteOptions, find_route_between_ptrs,
	};

	#[test]
	fn finds_two_hop_route_in_synthetic_chain() {
		let payload_a = 0x2000_u64.to_le_bytes();
		let payload_b = 0x3000_u64.to_le_bytes();
		let payload_c = 0_u64.to_le_bytes();

		let block_a = Block {
			head: BHead {
				code: *b"DATA",
				sdna_nr: 0,
				old: 0x1000,
				len: 8,
				nr: 1,
			},
			payload: &payload_a,
			file_offset: 0,
		};
		let block_b = Block {
			head: BHead {
				code: *b"DATA",
				sdna_nr: 0,
				old: 0x2000,
				len: 8,
				nr: 1,
			},
			payload: &payload_b,
			file_offset: 32,
		};
		let block_c = Block {
			head: BHead {
				code: *b"DATA",
				sdna_nr: 0,
				old: 0x3000,
				len: 8,
				nr: 1,
			},
			payload: &payload_c,
			file_offset: 64,
		};

		let index = PointerIndex::from_entries_for_test(vec![
			PtrEntry {
				start_old: 0x1000,
				end_old: 0x1008,
				block: block_a,
			},
			PtrEntry {
				start_old: 0x2000,
				end_old: 0x2008,
				block: block_b,
			},
			PtrEntry {
				start_old: 0x3000,
				end_old: 0x3008,
				block: block_c,
			},
		]);

		let dna = Dna {
			endianness: crate::blend::Endianness::Little,
			pointer_size: 8,
			names: vec!["*next".into()],
			types: vec!["Node".into()],
			tlen: vec![8],
			structs: vec![DnaStruct {
				type_idx: 0,
				fields: vec![DnaField { type_idx: 0, name_idx: 0 }],
			}],
			struct_for_type: vec![Some(0)],
		};

		let ids = IdIndex::build(vec![
			IdRecord {
				old_ptr: 0x1000,
				code: *b"A\0\0\0",
				sdna_nr: 0,
				type_name: "Node".into(),
				id_name: "AA".into(),
				next: None,
				prev: None,
				lib: None,
			},
			IdRecord {
				old_ptr: 0x2000,
				code: *b"B\0\0\0",
				sdna_nr: 0,
				type_name: "Node".into(),
				id_name: "BB".into(),
				next: None,
				prev: None,
				lib: None,
			},
			IdRecord {
				old_ptr: 0x3000,
				code: *b"C\0\0\0",
				sdna_nr: 0,
				type_name: "Node".into(),
				id_name: "CC".into(),
				next: None,
				prev: None,
				lib: None,
			},
		]);

		let result = find_route_between_ptrs(
			&dna,
			&index,
			&ids,
			0x1000,
			0x3000,
			&RouteOptions {
				max_depth: 3,
				max_nodes: 64,
				max_edges: 64,
				ref_scan: RefScanOptions {
					max_depth: 0,
					max_array_elems: 64,
				},
			},
		)
		.expect("route succeeds");

		let path = result.path.expect("path should be found");
		assert_eq!(path.len(), 2);
		assert_eq!(path[0].from, 0x1000);
		assert_eq!(path[0].to, 0x2000);
		assert_eq!(path[0].field.as_ref(), "next");
		assert_eq!(path[1].from, 0x2000);
		assert_eq!(path[1].to, 0x3000);
		assert_eq!(path[1].field.as_ref(), "next");
	}
}
