use std::path::PathBuf;

use blendoc::blend::{BlendFile, DecodeOptions, Value, chase_scene_camera};

use crate::cmd::util::render_code;

/// Resolve and print the active scene camera target, if present.
pub fn run(path: PathBuf) -> blendoc::blend::Result<()> {
	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;
	let index = blend.pointer_index()?;

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

	let Some((meta, object)) = chase_scene_camera(&blend, &dna, &index, &scene_decode, &object_decode)? else {
		println!("path: {}", path.display());
		println!("camera: null or unresolved");
		return Ok(());
	};

	println!("path: {}", path.display());
	println!("camera_ptr: 0x{:016x}", meta.ptr);
	println!("resolved_code: {}", render_code(meta.resolved_block_code));
	println!("resolved_sdna: {}", meta.sdna_nr);
	println!("element_index: {}", meta.element_index);
	println!("element_offset: {}", meta.element_offset);
	println!("object_type: {}", object.type_name);
	println!("object_fields: {}", object.fields.len());
	println!("object_preview:");
	for field in object.fields.iter().take(24) {
		println!("  {} = {}", field.name, brief_value(&field.value));
	}
	if object.fields.len() > 24 {
		println!("  ... {} more fields", object.fields.len() - 24);
	}

	Ok(())
}

fn brief_value(value: &Value) -> String {
	match value {
		Value::Null => "null".to_owned(),
		Value::Bool(v) => v.to_string(),
		Value::I64(v) => v.to_string(),
		Value::U64(v) => v.to_string(),
		Value::F32(v) => v.to_string(),
		Value::F64(v) => v.to_string(),
		Value::Bytes(v) => format!("bytes[{}]", v.len()),
		Value::String(v) => format!("\"{}\"", v),
		Value::Ptr(v) => format!("0x{v:016x}"),
		Value::Array(v) => format!("array[{}]", v.len()),
		Value::Struct(v) => format!("{}{{...}}", v.type_name),
	}
}
