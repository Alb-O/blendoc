#![allow(missing_docs)]

use std::path::{Path, PathBuf};

use blendoc::blend::{BlendFile, DecodeOptions, Value, decode_block_instances};

#[test]
fn character_glob_decode_smoke() {
	assert_glob_decode("character.blend");
}

#[test]
fn sword_glob_decode_smoke() {
	assert_glob_decode("sword.blend");
}

fn assert_glob_decode(name: &str) {
	let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
	let dna = blend.dna().expect("dna parses");
	let block = blend
		.find_first_block_by_code(*b"GLOB")
		.expect("block iteration succeeds")
		.expect("GLOB block exists");
	let value = decode_block_instances(&dna, &block, &DecodeOptions::default()).expect("decode succeeds");

	let strict = DecodeOptions {
		include_padding: true,
		strict_layout: true,
		..DecodeOptions::default()
	};
	decode_block_instances(&dna, &block, &strict).expect("strict decode succeeds");

	match value {
		Value::Struct(item) => {
			assert!(!item.type_name.is_empty(), "type name should exist");
			assert!(item.fields.len() > 5, "expected some fields");
		}
		Value::Array(items) => {
			assert!(!items.is_empty(), "expected at least one instance");
			let Some(Value::Struct(item)) = items.first() else {
				panic!("expected struct in array");
			};
			assert!(!item.type_name.is_empty(), "type name should exist");
			assert!(item.fields.len() > 5, "expected some fields");
		}
		_ => panic!("expected struct-like decode output"),
	}
}

fn fixture_path(name: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}
