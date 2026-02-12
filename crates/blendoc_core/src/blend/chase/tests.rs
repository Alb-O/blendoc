mod fixtures_day4_chase_camera {

	use blendoc_testkit::fixture_path;

	use crate::blend::{BlendFile, DecodeOptions, Value, chase_ptr_to_struct, chase_scene_camera, decode_block_instances};

	#[test]
	fn character_chase_camera_smoke() {
		assert_chase_camera("character.blend");
	}

	#[test]
	fn sword_chase_camera_smoke() {
		assert_chase_camera("sword.blend");
	}

	#[test]
	fn character_chase_world_smoke() {
		assert_chase_world("character.blend");
	}

	#[test]
	fn sword_chase_world_smoke() {
		assert_chase_world("sword.blend");
	}

	fn assert_chase_camera(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let index = blend.pointer_index().expect("pointer index builds");

		let scene_decode = DecodeOptions {
			include_padding: true,
			strict_layout: true,
			..DecodeOptions::for_scene_inspect()
		};

		let object_decode = DecodeOptions {
			max_depth: 6,
			include_padding: true,
			strict_layout: true,
			..DecodeOptions::default()
		};

		let scene_block = blend
			.find_first_block_by_code([b'S', b'C', 0, 0])
			.expect("scene block lookup")
			.expect("scene block exists");
		let scene_value = decode_block_instances(&dna, &scene_block, &scene_decode).expect("scene decode succeeds");
		let camera_ptr = extract_ptr_field(scene_value, "camera").expect("camera field exists");

		let chased = chase_scene_camera(&blend, &dna, &index, &scene_decode, &object_decode).expect("chase succeeds");

		if camera_ptr == 0 {
			assert!(chased.is_none(), "camera pointer is null, expected no chase result");
			return;
		}

		let chased = chased.expect("camera should resolve");

		assert_eq!(chased.1.type_name.as_ref(), "Object");
		assert_eq!(chased.0.element_offset, 0);
	}

	fn assert_chase_world(name: &str) {
		let blend = BlendFile::open(fixture_path(name)).expect("fixture opens");
		let dna = blend.dna().expect("dna parses");
		let index = blend.pointer_index().expect("pointer index builds");

		let scene_decode = DecodeOptions {
			include_padding: true,
			strict_layout: true,
			..DecodeOptions::for_scene_inspect()
		};

		let world_decode = DecodeOptions {
			include_padding: true,
			strict_layout: true,
			..DecodeOptions::default()
		};

		let scene_block = blend
			.find_first_block_by_code([b'S', b'C', 0, 0])
			.expect("scene block lookup")
			.expect("scene block exists");
		let scene_value = decode_block_instances(&dna, &scene_block, &scene_decode).expect("scene decode succeeds");
		let world_ptr = extract_ptr_field(scene_value, "world").expect("world field exists");
		assert_ne!(world_ptr, 0, "expected scene world pointer to exist");

		let chased = chase_ptr_to_struct(&dna, &index, world_ptr, &world_decode)
			.expect("chase succeeds")
			.expect("world should resolve");
		assert_eq!(chased.1.type_name.as_ref(), "World");
		assert_eq!(chased.0.element_offset, 0);
	}

	fn extract_ptr_field(value: Value, field_name: &str) -> Option<u64> {
		let Value::Struct(item) = value else {
			return None;
		};
		let field = item.fields.iter().find(|field| field.name.as_ref() == field_name)?;
		let Value::Ptr(ptr) = field.value else {
			return None;
		};
		Some(ptr)
	}
}
