mod fixtures_day5_ids {

	use std::path::{Path, PathBuf};

	use crate::blend::{BlendFile, scan_id_blocks};

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
}

mod id_root_detection {
	use super::super::id_root_flags;
	use crate::blend::{Dna, DnaField, DnaStruct};

	#[test]
	fn id_root_detection_handles_non_id_roots() {
		let dna = Dna {
			names: vec!["id".into(), "other".into()],
			types: vec!["ID".into(), "Scene".into(), "NoIdRoot".into()],
			tlen: vec![8, 24, 16],
			structs: vec![
				DnaStruct {
					type_idx: 0,
					fields: vec![DnaField { type_idx: 0, name_idx: 1 }],
				},
				DnaStruct {
					type_idx: 1,
					fields: vec![DnaField { type_idx: 0, name_idx: 0 }],
				},
				DnaStruct {
					type_idx: 2,
					fields: vec![DnaField { type_idx: 2, name_idx: 1 }],
				},
			],
			struct_for_type: vec![Some(0), Some(1), Some(2)],
		};

		let roots = id_root_flags(&dna);
		assert_eq!(roots, vec![false, true, false]);
	}
}
