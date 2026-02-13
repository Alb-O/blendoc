mod fixtures_day3_decode {

	use blendoc_testkit::fixture_path;

	use crate::blend::{BlendFile, DecodeOptions, Value, decode_block_instances};

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
}

mod fixtures_day3_scene {

	use blendoc_testkit::fixture_path;

	use crate::blend::{BlendFile, DecodeOptions, Value, decode_block_instances};

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
}

mod fixtures_day12_show {

	use blendoc_testkit::fixture_path;

	use crate::blend::{BlendFile, DecodeOptions, IdIndex, decode_ptr_instance, scan_id_blocks};

	#[test]
	fn character_decode_ptr_instance_world() {
		assert_world_decode("character.blend");
	}

	#[test]
	fn sword_decode_ptr_instance_world() {
		assert_world_decode("sword.blend");
	}

	#[test]
	fn character_v51_decode_ptr_instance_world() {
		assert_world_decode("v5.1_character.blend");
	}

	#[test]
	fn sword_v51_decode_ptr_instance_world() {
		assert_world_decode("v5.1_sword.blend");
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
}

mod unit_big_endian_decode {
	use crate::blend::{BHead, Block, DecodeOptions, Dna, DnaField, DnaStruct, Endianness, Value, decode_block_instances};

	#[test]
	fn decodes_big_endian_numeric_and_pointer_fields() {
		let dna = Dna {
			endianness: Endianness::Big,
			pointer_size: 8,
			names: vec!["value".into(), "*next".into()],
			types: vec!["uint".into(), "Node".into()],
			tlen: vec![4, 12],
			structs: vec![DnaStruct {
				type_idx: 1,
				fields: vec![DnaField { type_idx: 0, name_idx: 0 }, DnaField { type_idx: 1, name_idx: 1 }],
			}],
			struct_for_type: vec![None, Some(0)],
		};

		let mut payload = Vec::new();
		payload.extend_from_slice(&0x0102_0304_u32.to_be_bytes());
		payload.extend_from_slice(&0x1122_3344_5566_7788_u64.to_be_bytes());
		let block = Block {
			head: BHead {
				code: *b"DATA",
				sdna_nr: 0,
				old: 0x1000,
				len: payload.len() as u64,
				nr: 1,
			},
			payload: &payload,
			file_offset: 0,
		};

		let value = decode_block_instances(&dna, &block, &DecodeOptions::default()).expect("decode succeeds");
		let Value::Struct(item) = value else {
			panic!("expected struct decode");
		};
		assert_eq!(item.type_name.as_ref(), "Node");

		let number = item.fields.iter().find(|field| field.name.as_ref() == "value").expect("value field exists");
		assert!(matches!(number.value, Value::U64(0x0102_0304)));

		let next = item.fields.iter().find(|field| field.name.as_ref() == "next").expect("next field exists");
		assert!(matches!(next.value, Value::Ptr(0x1122_3344_5566_7788)));
	}
}
