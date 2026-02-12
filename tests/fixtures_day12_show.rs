#![allow(missing_docs)]

use std::path::{Path, PathBuf};

use blendoc::blend::{BlendFile, DecodeOptions, IdIndex, decode_ptr_instance, scan_id_blocks};

#[test]
fn character_decode_ptr_instance_world() {
	assert_world_decode("character.blend");
}

#[test]
fn sword_decode_ptr_instance_world() {
	assert_world_decode("sword.blend");
}

fn assert_world_decode(name: &str) {
	let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
	let dna = blend.dna().expect("dna parses");
	let index = blend.pointer_index().expect("pointer index builds");
	let ids = IdIndex::build(scan_id_blocks(&blend, &dna).expect("id scan succeeds"));

	let world = ids.get_by_name("WOWorld").expect("world id record exists");
	let (canonical, value) = decode_ptr_instance(&dna, &index, world.old_ptr, &DecodeOptions::default()).expect("decode ptr instance succeeds");

	assert_eq!(canonical, world.old_ptr);
	assert_eq!(value.type_name.as_ref(), "World");
}

fn fixture_path(name: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}
