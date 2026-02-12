mod fixtures_day7_refs {

	use blendoc_testkit::fixture_path;

	use crate::blend::{BlendFile, IdIndex, RefScanOptions, scan_id_blocks, scan_refs_from_ptr};

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
}

mod pointer_arrays {
	use crate::blend::{BHead, Block, Dna, DnaField, DnaStruct, IdIndex, PointerIndex, PtrEntry, RefScanOptions, scan_refs_from_ptr};

	#[test]
	fn pointer_arrays_and_nested_depth_one_are_scanned() {
		let mut owner_payload = [0_u8; 24];
		owner_payload[0..8].copy_from_slice(&0x2000_u64.to_le_bytes());
		owner_payload[8..16].copy_from_slice(&0_u64.to_le_bytes());
		owner_payload[16..24].copy_from_slice(&0x3000_u64.to_le_bytes());

		let nested_payload = 0_u64.to_le_bytes();

		let root_block = Block {
			head: BHead {
				code: *b"ROOT",
				sdna_nr: 0,
				old: 0x1000,
				len: owner_payload.len() as u64,
				nr: 1,
			},
			payload: &owner_payload,
			file_offset: 0,
		};

		let target_a = Block {
			head: BHead {
				code: *b"DATA",
				sdna_nr: 1,
				old: 0x2000,
				len: nested_payload.len() as u64,
				nr: 1,
			},
			payload: &nested_payload,
			file_offset: 32,
		};

		let target_b = Block {
			head: BHead {
				code: *b"DATA",
				sdna_nr: 1,
				old: 0x3000,
				len: nested_payload.len() as u64,
				nr: 1,
			},
			payload: &nested_payload,
			file_offset: 64,
		};

		let index = PointerIndex::from_entries_for_test(vec![
			PtrEntry {
				start_old: 0x1000,
				end_old: 0x1000 + owner_payload.len() as u64,
				block: root_block,
			},
			PtrEntry {
				start_old: 0x2000,
				end_old: 0x2000 + nested_payload.len() as u64,
				block: target_a,
			},
			PtrEntry {
				start_old: 0x3000,
				end_old: 0x3000 + nested_payload.len() as u64,
				block: target_b,
			},
		]);

		let dna = Dna {
			names: vec!["*arr[2]".into(), "nested".into(), "*first".into()],
			types: vec!["Owner".into(), "Nested".into()],
			tlen: vec![24, 8],
			structs: vec![
				DnaStruct {
					type_idx: 0,
					fields: vec![DnaField { type_idx: 1, name_idx: 0 }, DnaField { type_idx: 1, name_idx: 1 }],
				},
				DnaStruct {
					type_idx: 1,
					fields: vec![DnaField { type_idx: 1, name_idx: 2 }],
				},
			],
			struct_for_type: vec![Some(0), Some(1)],
		};

		let id_index = IdIndex::build(Vec::new());
		let refs = scan_refs_from_ptr(
			&dna,
			&index,
			&id_index,
			0x1000,
			&RefScanOptions {
				max_depth: 1,
				max_array_elems: 16,
			},
		)
		.expect("scan succeeds");

		assert!(refs.iter().any(|item| item.field.as_ref() == "arr[0]" && item.ptr == 0x2000));
		assert!(refs.iter().any(|item| item.field.as_ref() == "arr[1]" && item.ptr == 0));
		assert!(refs.iter().any(|item| item.field.as_ref() == "nested.first" && item.ptr == 0x3000));
	}
}

mod stable_ids {
	use blendoc_testkit::fixture_path;

	use crate::blend::{BlendFile, IdIndex, PointerStorage, RefScanOptions, scan_id_blocks, scan_refs_from_ptr};

	#[test]
	fn v51_character_scene_refs_resolve_exact_ids_only() {
		assert_exact_stable_id_resolution("v5.1_character.blend");
	}

	#[test]
	fn v51_sword_scene_refs_resolve_exact_ids_only() {
		assert_exact_stable_id_resolution("v5.1_sword.blend");
	}

	fn assert_exact_stable_id_resolution(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let index = blend.pointer_index().expect("pointer index builds");
		assert_eq!(index.storage(), PointerStorage::StableIds);

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

		for record in refs.iter().filter(|item| item.ptr != 0) {
			if let Some(target) = &record.resolved {
				assert_eq!(
					target.canonical, record.ptr,
					"stable-id resolution should require exact-id match (field={})",
					record.field
				);
			}
		}
	}
}
