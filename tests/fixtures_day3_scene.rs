#![allow(missing_docs)]

use std::path::{Path, PathBuf};

use blendoc::blend::{BlendFile, DecodeOptions, Value, decode_block_instances};

#[test]
fn character_scene_decode_smoke() {
	assert_scene_decode("character.blend");
}

#[test]
fn sword_scene_decode_smoke() {
	assert_scene_decode("sword.blend");
}

fn assert_scene_decode(name: &str) {
	let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
	let dna = blend.dna().expect("dna parses");
	let block = blend
		.find_first_block_by_code([b'S', b'C', 0, 0])
		.expect("block iteration succeeds")
		.expect("SC block exists");
	let value = decode_block_instances(&dna, &block, &DecodeOptions::for_scene_inspect()).expect("decode succeeds");

	let mut strict = DecodeOptions::for_scene_inspect();
	strict.include_padding = true;
	strict.strict_layout = true;
	decode_block_instances(&dna, &block, &strict).expect("strict decode succeeds");

	let item = match value {
		Value::Struct(item) => item,
		Value::Array(mut items) => {
			let Some(Value::Struct(item)) = items.pop() else {
				panic!("expected struct in array");
			};
			item
		}
		_ => panic!("expected struct-like value"),
	};

	assert_eq!(item.type_name.as_ref(), "Scene");
	assert!(item.fields.len() > 20, "expected many scene fields");
}

fn fixture_path(name: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}
