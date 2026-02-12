mod fixtures_day9_xref {

	use blendoc_testkit::fixture_path;

	use crate::blend::{BlendFile, IdIndex, RefScanOptions, XrefOptions, find_inbound_refs_to_ptr, scan_id_blocks};

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
}

mod nested_inbound {
	use crate::blend::{
		BHead, Block, Dna, DnaField, DnaStruct, IdIndex, IdRecord, PointerIndex, PtrEntry, RefScanOptions, XrefOptions, find_inbound_refs_to_ptr,
	};

	#[test]
	fn nested_field_inbound_reference_is_reported() {
		let mut owner_payload = [0_u8; 16];
		owner_payload[8..16].copy_from_slice(&0x3000_u64.to_le_bytes());
		let target_payload = [0_u8; 8];

		let owner_block = Block {
			head: BHead {
				code: *b"SC\0\0",
				sdna_nr: 0,
				old: 0x1000,
				len: owner_payload.len() as u64,
				nr: 1,
			},
			payload: &owner_payload,
			file_offset: 0,
		};
		let target_block = Block {
			head: BHead {
				code: *b"WO\0\0",
				sdna_nr: 1,
				old: 0x3000,
				len: target_payload.len() as u64,
				nr: 1,
			},
			payload: &target_payload,
			file_offset: 64,
		};

		let index = PointerIndex::from_entries_for_test(vec![
			PtrEntry {
				start_old: 0x1000,
				end_old: 0x1010,
				block: owner_block,
			},
			PtrEntry {
				start_old: 0x3000,
				end_old: 0x3008,
				block: target_block,
			},
		]);

		let dna = Dna {
			names: vec!["id[8]".into(), "nested".into(), "*first".into()],
			types: vec!["char".into(), "Owner".into(), "Nested".into(), "Target".into()],
			tlen: vec![1, 16, 8, 8],
			structs: vec![
				DnaStruct {
					type_idx: 1,
					fields: vec![DnaField { type_idx: 0, name_idx: 0 }, DnaField { type_idx: 2, name_idx: 1 }],
				},
				DnaStruct {
					type_idx: 2,
					fields: vec![DnaField { type_idx: 3, name_idx: 2 }],
				},
				DnaStruct { type_idx: 3, fields: vec![] },
			],
			struct_for_type: vec![None, Some(0), Some(1), Some(2)],
		};

		let ids = IdIndex::build(vec![IdRecord {
			old_ptr: 0x1000,
			code: *b"SC\0\0",
			sdna_nr: 0,
			type_name: "Owner".into(),
			id_name: "SCOwner".into(),
			next: None,
			prev: None,
			lib: None,
		}]);

		let refs = find_inbound_refs_to_ptr(
			&dna,
			&index,
			&ids,
			0x3000,
			&XrefOptions {
				ref_scan: RefScanOptions {
					max_depth: 1,
					max_array_elems: 64,
				},
				max_results: 32,
				include_unresolved: false,
			},
		)
		.expect("xref succeeds");

		assert!(refs.iter().any(|item| item.from == 0x1000 && item.field.as_ref() == "nested.first"));
	}
}
