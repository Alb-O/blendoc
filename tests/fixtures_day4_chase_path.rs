#![allow(missing_docs)]

use std::path::{Path, PathBuf};

use blendoc::blend::{BlendFile, ChasePolicy, ChaseStopReason, DecodeOptions, FieldPath, Value, chase_from_block_code};

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
