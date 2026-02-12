#![allow(missing_docs)]

use std::path::{Path, PathBuf};

use blendoc::blend::{BlendFile, scan_id_blocks};

#[test]
fn character_ids_scan_smoke() {
	assert_ids_scan("character.blend");
}

#[test]
fn sword_ids_scan_smoke() {
	assert_ids_scan("sword.blend");
}

fn assert_ids_scan(name: &str) {
	let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
	let dna = blend.dna().expect("dna parses");
	let rows = scan_id_blocks(&blend, &dna).expect("id scan succeeds");

	assert!(!rows.is_empty(), "expected at least one ID-root block");
	assert!(
		rows.iter().any(|row| row.code == [b'S', b'C', 0, 0] || row.type_name.as_ref() == "Scene"),
		"expected Scene-derived ID entry"
	);
	assert!(rows.iter().all(|row| !row.id_name.trim().is_empty()), "expected non-empty ID names");
}

fn fixture_path(name: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}
