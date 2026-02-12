mod fixture_provenance {
	use std::path::{Path, PathBuf};

	use crate::blend::{BlendFile, LinkConfidence, scan_id_link_provenance, scan_library_records};

	#[test]
	fn character_declares_linked_sword_library() {
		assert_character_has_sword_library("character.blend");
	}

	#[test]
	fn character_v51_declares_linked_sword_library() {
		assert_character_has_sword_library("v5.1_character.blend");
	}

	#[test]
	fn sword_has_no_library_records() {
		assert_no_library_records("sword.blend");
	}

	#[test]
	fn sword_v51_has_no_library_records() {
		assert_no_library_records("v5.1_sword.blend");
	}

	#[test]
	fn linked_object_ranks_higher_than_local_object_in_character() {
		assert_link_confidence_order("character.blend");
	}

	#[test]
	fn linked_object_ranks_higher_than_local_object_in_character_v51() {
		assert_link_confidence_order("v5.1_character.blend");
	}

	fn assert_character_has_sword_library(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let libraries = scan_library_records(&blend, &dna).expect("library scan succeeds");

		assert!(!libraries.is_empty(), "expected at least one Library record");
		assert!(
			libraries.iter().any(|item| item.library_path.contains("sword.blend") && item.is_relative),
			"expected a relative linked sword library path"
		);
	}

	fn assert_no_library_records(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let libraries = scan_library_records(&blend, &dna).expect("library scan succeeds");
		assert!(libraries.is_empty(), "expected no Library records in source sword file");
	}

	fn assert_link_confidence_order(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let provenance = scan_id_link_provenance(&blend, &dna).expect("provenance scan succeeds");

		let sword = provenance
			.iter()
			.find(|item| item.id_name.as_ref() == "OBsword_object")
			.expect("linked sword object provenance exists");
		let character = provenance
			.iter()
			.find(|item| item.id_name.as_ref() == "OBcharacter_model")
			.expect("local character object provenance exists");

		assert!(sword.confidence.rank() >= LinkConfidence::Medium.rank());
		assert!(sword.linked, "linked object should be marked linked");
		assert!(
			character.confidence.rank() < sword.confidence.rank(),
			"local object should have lower link confidence than linked sword object"
		);
	}

	fn fixture_path(name: &str) -> PathBuf {
		Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
	}
}
