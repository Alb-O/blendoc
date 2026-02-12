mod fixtures_day4_chase_path {

	use std::path::{Path, PathBuf};

	use crate::blend::{BlendFile, ChasePolicy, ChaseStopReason, DecodeOptions, FieldPath, Value, chase_from_block_code};

	#[test]
	fn character_world_path_chase() {
		assert_world_path("character.blend");
	}

	#[test]
	fn sword_world_path_chase() {
		assert_world_path("sword.blend");
	}

	#[test]
	fn character_view_layer_path_chase() {
		assert_view_layer_path("character.blend");
	}

	#[test]
	fn sword_view_layer_path_chase() {
		assert_view_layer_path("sword.blend");
	}

	fn assert_world_path(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let index = blend.pointer_index().expect("pointer index builds");

		let mut decode = DecodeOptions::for_scene_inspect();
		decode.include_padding = true;
		decode.strict_layout = true;

		let path = FieldPath::parse("world").expect("path parses");
		let result = chase_from_block_code(&blend, &dna, &index, [b'S', b'C', 0, 0], &path, &decode, &ChasePolicy::default()).expect("chase succeeds");

		assert!(result.stop.is_none(), "expected world path to resolve");
		let Value::Struct(item) = result.value else {
			panic!("expected struct world result");
		};
		assert_eq!(item.type_name.as_ref(), "World");
	}

	fn assert_view_layer_path(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let index = blend.pointer_index().expect("pointer index builds");

		let mut decode = DecodeOptions::for_scene_inspect();
		decode.include_padding = true;
		decode.strict_layout = true;

		let path = FieldPath::parse("view_layers.first").expect("path parses");
		let result = chase_from_block_code(&blend, &dna, &index, [b'S', b'C', 0, 0], &path, &decode, &ChasePolicy::default()).expect("chase succeeds");

		if let Some(stop) = result.stop {
			match stop.reason {
				ChaseStopReason::NullPtr | ChaseStopReason::UnresolvedPtr(_) => {}
				other => panic!("unexpected stop reason: {other:?}"),
			}
			return;
		}

		let Value::Struct(item) = result.value else {
			panic!("expected view layer struct result");
		};
		assert_eq!(item.type_name.as_ref(), "ViewLayer");
	}

	fn fixture_path(name: &str) -> PathBuf {
		Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
	}
}

mod fixtures_day6_chase_ids {

	use std::collections::HashMap;
	use std::path::{Path, PathBuf};

	use crate::blend::{BlendFile, ChaseMeta, ChasePolicy, DecodeOptions, FieldPath, Value, chase_from_ptr, scan_id_blocks};

	#[test]
	fn character_scene_world_chase_has_id_annotations() {
		assert_scene_world_chase("character.blend");
	}

	#[test]
	fn sword_scene_world_chase_has_id_annotations() {
		assert_scene_world_chase("sword.blend");
	}

	fn assert_scene_world_chase(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let index = blend.pointer_index().expect("pointer index builds");
		let ids = scan_id_blocks(&blend, &dna).expect("id scan succeeds");

		let scene = ids.iter().find(|item| item.code == [b'S', b'C', 0, 0]).expect("scene ID record exists");

		let id_by_ptr: HashMap<u64, &str> = ids.iter().map(|item| (item.old_ptr, item.id_name.as_ref())).collect();

		let mut decode = DecodeOptions::for_scene_inspect();
		decode.include_padding = true;
		decode.strict_layout = true;

		let path = FieldPath::parse("world").expect("path parses");
		let result = chase_from_ptr(&dna, &index, scene.old_ptr, &path, &decode, &ChasePolicy::default()).expect("chase succeeds");

		assert!(result.stop.is_none(), "world path should resolve cleanly");
		let Value::Struct(item) = result.value else {
			panic!("expected world struct result")
		};
		assert_eq!(item.type_name.as_ref(), "World");

		let scene_name = id_by_ptr.get(&scene.old_ptr).expect("scene id annotation exists");
		assert!(scene_name.starts_with("SC"), "expected Scene ID prefix");

		let hop_id_names: Vec<&str> = result
			.hops
			.iter()
			.filter_map(|hop| canonical_ptr(hop).and_then(|ptr| id_by_ptr.get(&ptr).copied()))
			.collect();
		assert!(hop_id_names.iter().any(|id_name| id_name.starts_with("WO")), "expected World hop ID annotation");
	}

	fn canonical_ptr(hop: &ChaseMeta) -> Option<u64> {
		let offset = hop.element_index.checked_mul(hop.struct_size)?;
		let offset = u64::try_from(offset).ok()?;
		hop.block_old.checked_add(offset)
	}

	fn fixture_path(name: &str) -> PathBuf {
		Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
	}
}

mod unit_chase_cycle {

	use crate::blend::{BHead, ChasePolicy, ChaseStopReason, Dna, DnaField, DnaStruct, FieldPath, PointerIndex, PtrEntry, StopMode, chase_from_ptr};

	#[test]
	fn cycle_is_detected_and_stops() {
		let payload_a = 0x2000_u64.to_le_bytes();
		let payload_b = 0x1000_u64.to_le_bytes();

		let block_a = crate::blend::Block {
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

		let block_b = crate::blend::Block {
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
		let policy = ChasePolicy {
			on_cycle: StopMode::Stop,
			..ChasePolicy::default()
		};

		let result = chase_from_ptr(&dna, &index, 0x1000, &path, &crate::blend::DecodeOptions::default(), &policy).expect("chase succeeds");

		let stop = result.stop.expect("expected stop");
		assert!(matches!(stop.reason, ChaseStopReason::Cycle(_)));
		assert_eq!(result.hops.len(), 2);
	}
}
