#![allow(missing_docs)]

use std::sync::Arc;

use blendoc::blend::{
	BHead, Block, Dna, DnaField, DnaStruct, IdIndex, IdRecord, PointerIndex, PtrEntry, RefScanOptions, StopMode, WalkOptions, WalkStopReason, walk_ptr_chain,
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
