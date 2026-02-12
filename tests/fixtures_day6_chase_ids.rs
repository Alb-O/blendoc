#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use blendoc::blend::{BlendFile, ChaseMeta, ChasePolicy, DecodeOptions, FieldPath, Value, chase_from_ptr, scan_id_blocks};

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
