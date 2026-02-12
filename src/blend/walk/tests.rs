mod fixtures_day13_walk {

	use std::path::{Path, PathBuf};
	use std::sync::Arc;

	use crate::blend::{BlendFile, IdIndex, RefScanOptions, StopMode, WalkOptions, scan_id_blocks, walk_ptr_chain};

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
}

mod unit_walk_chain {

	use std::sync::Arc;

	use crate::blend::{
		BHead, Block, Dna, DnaField, DnaStruct, IdIndex, IdRecord, PointerIndex, PtrEntry, RefScanOptions, StopMode, WalkOptions, WalkStopReason,
		walk_ptr_chain,
	};

	#[test]
	fn walk_follows_three_node_chain_and_stops_on_null() {
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
				code: *b"AA\0\0",
				sdna_nr: 0,
				type_name: "Node".into(),
				id_name: "AANode".into(),
				next: None,
				prev: None,
				lib: None,
			},
			IdRecord {
				old_ptr: 0x2000,
				code: *b"BB\0\0",
				sdna_nr: 0,
				type_name: "Node".into(),
				id_name: "BBNode".into(),
				next: None,
				prev: None,
				lib: None,
			},
			IdRecord {
				old_ptr: 0x3000,
				code: *b"CC\0\0",
				sdna_nr: 0,
				type_name: "Node".into(),
				id_name: "CCNode".into(),
				next: None,
				prev: None,
				lib: None,
			},
		]);

		let result = walk_ptr_chain(
			&dna,
			&index,
			&ids,
			0x1000,
			&WalkOptions {
				next_field: Arc::<str>::from("next"),
				max_steps: 10,
				ref_scan: RefScanOptions {
					max_depth: 0,
					max_array_elems: 32,
				},
				on_null: StopMode::Stop,
				on_unresolved: StopMode::Stop,
				on_cycle: StopMode::Stop,
			},
		)
		.expect("walk succeeds");

		assert_eq!(result.items.len(), 3);
		let stop = result.stop.expect("expected stop");
		assert_eq!(stop.step, 2);
		assert!(matches!(stop.reason, WalkStopReason::NullNext));
	}
}
