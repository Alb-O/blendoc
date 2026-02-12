mod fixtures_day2_dna {

	use blendoc_testkit::fixture_path;

	use crate::blend::BlendFile;

	#[test]
	fn character_fixture_dna_smoke() {
		assert_fixture_dna("character.blend");
	}

	#[test]
	fn sword_fixture_dna_smoke() {
		assert_fixture_dna("sword.blend");
	}

	fn assert_fixture_dna(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");

		assert!(dna.names.len() > 100, "expected name table");
		assert!(dna.types.len() > 50, "expected type table");
		assert!(dna.structs.len() > 50, "expected struct table");
		assert_eq!(dna.tlen.len(), dna.types.len(), "TLEN must match TYPE count");
		assert_eq!(dna.struct_for_type.len(), dna.types.len(), "lookup size must match TYPE count");

		for item in &dna.structs {
			assert!((item.type_idx as usize) < dna.types.len(), "struct type idx in range");
			for field in &item.fields {
				assert!((field.type_idx as usize) < dna.types.len(), "field type idx in range");
				assert!((field.name_idx as usize) < dna.names.len(), "field name idx in range");
			}
		}
	}
}
