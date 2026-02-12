mod fixtures_day4_pointers {

	use std::path::{Path, PathBuf};

	use crate::blend::BlendFile;

	#[test]
	fn character_pointer_index_smoke() {
		assert_pointer_index("character.blend");
	}

	#[test]
	fn sword_pointer_index_smoke() {
		assert_pointer_index("sword.blend");
	}

	fn assert_pointer_index(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let index = blend.pointer_index().expect("pointer index builds");

		assert!(index.len() > 100, "expected many pointer entries");

		for entry in index.entries().iter().take(100) {
			let base = index.resolve(entry.start_old).expect("base pointer resolves");
			assert_eq!(base.byte_offset, 0);

			if entry.end_old > entry.start_old + 8 {
				let inside = index.resolve(entry.start_old + 8).expect("in-block pointer resolves");
				assert_eq!(inside.byte_offset, 8);
			}
		}

		let candidate = index.entries().iter().find(|entry| {
			if entry.block.head.nr < 2 {
				return false;
			}

			let Some(item) = dna.struct_by_sdna(entry.block.head.sdna_nr) else {
				return false;
			};

			let struct_size = usize::from(dna.tlen[item.type_idx as usize]);
			if struct_size == 0 {
				return false;
			}

			let Ok(nr) = usize::try_from(entry.block.head.nr) else {
				return false;
			};

			nr >= 2 && entry.block.payload.len() >= struct_size.saturating_mul(2)
		});

		let candidate = candidate.expect("candidate block for typed resolution");
		let item = dna.struct_by_sdna(candidate.block.head.sdna_nr).expect("sdna exists");
		let struct_size = usize::from(dna.tlen[item.type_idx as usize]);
		let ptr = candidate.start_old + struct_size as u64;

		let typed = index.resolve_typed(&dna, ptr).expect("typed pointer resolution works");
		assert_eq!(typed.element_index, Some(1));
		assert_eq!(typed.element_offset, 0);
	}

	fn fixture_path(name: &str) -> PathBuf {
		Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
	}
}
