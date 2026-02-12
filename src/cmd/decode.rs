use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, DecodeOptions, Value, decode_block_instances};

/// Output truncation and formatting limits for decoded values.
#[derive(Debug, Clone, Copy)]
pub struct PrintOptions {
	/// Maximum number of fields printed for a single struct.
	pub max_fields_per_struct: usize,
	/// Maximum number of Unicode scalar values printed for strings.
	pub max_string_len: usize,
	/// Maximum number of elements printed for arrays.
	pub max_array_items: usize,
	/// Maximum recursive print depth for nested arrays/structs.
	pub max_print_depth: u32,
}

impl Default for PrintOptions {
	fn default() -> Self {
		Self {
			max_fields_per_struct: 80,
			max_string_len: 200,
			max_array_items: 16,
			max_print_depth: 6,
		}
	}
}

impl PrintOptions {
	/// Preset tuned for scene-sized output.
	pub fn for_scene_inspect() -> Self {
		Self {
			max_fields_per_struct: 40,
			max_string_len: 160,
			max_array_items: 8,
			max_print_depth: 4,
		}
	}
}

/// Decode and print the first block matching `code`.
pub fn run(path: PathBuf, code: String) -> blendoc::blend::Result<()> {
	let block_code = parse_block_code(&code)?;
	run_with_code(path, block_code, DecodeOptions::default(), PrintOptions::default())
}

/// Decode and print the first block matching a binary block code.
pub fn run_with_code(path: PathBuf, block_code: [u8; 4], decode_options: DecodeOptions, print_options: PrintOptions) -> blendoc::blend::Result<()> {
	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;
	let block = blend
		.find_first_block_by_code(block_code)?
		.ok_or(BlendError::BlockNotFound { code: block_code })?;
	let value = decode_block_instances(&dna, &block, &decode_options)?;

	println!("path: {}", path.display());
	println!("code: {}", render_code(block_code));
	println!("sdna_nr: {}", block.head.sdna_nr);
	println!("nr: {}", block.head.nr);
	println!("len: {}", block.head.len);
	println!("decoded:");
	print_value(&value, 0, 0, print_options);

	Ok(())
}

fn parse_block_code(code: &str) -> blendoc::blend::Result<[u8; 4]> {
	if code.is_empty() || code.len() > 4 || !code.is_ascii() {
		return Err(BlendError::InvalidBlockCode { code: code.to_owned() });
	}

	let mut out = [0_u8; 4];
	out[..code.len()].copy_from_slice(code.as_bytes());
	Ok(out)
}

fn render_code(code: [u8; 4]) -> String {
	code.into_iter().filter(|byte| *byte != 0).map(char::from).collect()
}

fn print_value(value: &Value, indent: usize, depth: u32, options: PrintOptions) {
	let pad = " ".repeat(indent);
	match value {
		Value::Null => println!("{}null", pad),
		Value::Bool(v) => println!("{}{v}", pad),
		Value::I64(v) => println!("{}{v}", pad),
		Value::U64(v) => println!("{}{v}", pad),
		Value::F32(v) => println!("{}{v}", pad),
		Value::F64(v) => println!("{}{v}", pad),
		Value::Bytes(v) => println!("{}bytes[{}]", pad, v.len()),
		Value::String(v) => println!("{}\"{}\"", pad, truncate(v, options.max_string_len)),
		Value::Ptr(v) => println!("{}0x{v:016x}", pad),
		Value::Array(items) => {
			if depth >= options.max_print_depth {
				println!("{}[... {} items]", pad, items.len());
				return;
			}
			println!("{}[", pad);
			for item in items.iter().take(options.max_array_items) {
				print_value(item, indent + 2, depth + 1, options);
			}
			if items.len() > options.max_array_items {
				println!("{}  ... {} more", pad, items.len() - options.max_array_items);
			}
			println!("{}]", pad);
		}
		Value::Struct(item) => {
			if depth >= options.max_print_depth {
				println!("{}{} {{ ... }}", pad, item.type_name);
				return;
			}
			println!("{}{} {{", pad, item.type_name);
			for field in item.fields.iter().take(options.max_fields_per_struct) {
				print!("{}  {} = ", pad, field.name);
				if matches!(field.value, Value::Struct(_) | Value::Array(_)) {
					println!();
					print_value(&field.value, indent + 4, depth + 1, options);
				} else {
					print_value(&field.value, 0, depth + 1, options);
				}
			}
			if item.fields.len() > options.max_fields_per_struct {
				println!("{}  ... {} more fields", pad, item.fields.len() - options.max_fields_per_struct);
			}
			println!("{}}}", pad);
		}
	}
}

fn truncate(input: &str, max_len: usize) -> String {
	if input.chars().count() <= max_len {
		return input.to_owned();
	}
	let out: String = input.chars().take(max_len).collect();
	format!("{out}...")
}
