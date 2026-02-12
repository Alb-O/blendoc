#![allow(missing_docs)]

use blendoc::blend::{BHead, ChasePolicy, ChaseStopReason, Dna, DnaField, DnaStruct, FieldPath, PointerIndex, PtrEntry, StopMode, chase_from_ptr};

#[test]
fn cycle_is_detected_and_stops() {
	let payload_a = 0x2000_u64.to_le_bytes();
	let payload_b = 0x1000_u64.to_le_bytes();

	let block_a = blendoc::blend::Block {
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

	let block_b = blendoc::blend::Block {
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

	let path = FieldPath::parse("next.next.next").expect("path parses");
	let mut policy = ChasePolicy::default();
	policy.on_cycle = StopMode::Stop;

	let result = chase_from_ptr(&dna, &index, 0x1000, &path, &blendoc::blend::DecodeOptions::default(), &policy).expect("chase succeeds");

	let stop = result.stop.expect("expected stop");
	assert!(matches!(stop.reason, ChaseStopReason::Cycle(_)));
	assert_eq!(result.hops.len(), 2);
}
