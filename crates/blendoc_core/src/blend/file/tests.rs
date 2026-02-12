mod fixtures_day1 {

	use std::path::{Path, PathBuf};

	use crate::blend::{BlendFile, Compression};

	#[test]
	fn character_fixture_smoke() {
		assert_fixture("character.blend");
	}

	#[test]
	fn sword_fixture_smoke() {
		assert_fixture("sword.blend");
	}

	fn assert_fixture(name: &str) {
		let path = fixture_path(name);
		let blend = BlendFile::open(path).expect("fixture opens");
		let stats = blend.scan_block_stats().expect("scan succeeds");

		assert_eq!(blend.compression, Compression::Zstd);
		assert_eq!(blend.header.header_size, 17);
		assert_eq!(blend.header.format_version, 1);
		assert!(blend.header.version >= 500, "expected version >= 500");
		assert!(stats.block_count > 10, "expected enough blocks");
		assert!(stats.has_dna1, "expected DNA1 block");
		assert!(stats.has_endb, "expected ENDB block");
		assert_eq!(stats.last_code, *b"ENDB");
	}

	fn fixture_path(name: &str) -> PathBuf {
		Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("fixtures").join(name)
	}
}
