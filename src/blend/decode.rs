use crate::blend::bytes::Cursor;
use crate::blend::value::{FieldValue, StructValue, Value};
use crate::blend::{BlendError, Block, Dna, Result};

const POINTER_SIZE: usize = 8;

#[derive(Debug, Clone)]
pub struct DecodeOptions {
	pub max_depth: u32,
	pub max_array_elems: usize,
	pub include_padding: bool,
	pub decode_char_arrays_as_string: bool,
	pub strict_layout: bool,
}

impl Default for DecodeOptions {
	fn default() -> Self {
		Self {
			max_depth: 16,
			max_array_elems: 4096,
			include_padding: false,
			decode_char_arrays_as_string: true,
			strict_layout: false,
		}
	}
}

impl DecodeOptions {
	pub fn for_scene_inspect() -> Self {
		Self {
			max_depth: 8,
			max_array_elems: 4096,
			include_padding: false,
			decode_char_arrays_as_string: true,
			strict_layout: false,
		}
	}
}

pub fn decode_block_instances(dna: &Dna, block: &Block<'_>, opt: &DecodeOptions) -> Result<Value> {
	let sdna_nr = block.head.sdna_nr;
	let struct_def = dna.struct_by_sdna(sdna_nr).ok_or(BlendError::DecodeMissingSdna { sdna_nr })?;
	let struct_size = usize::from(dna.tlen[struct_def.type_idx as usize]);

	let count = usize::try_from(block.head.nr).map_err(|_| BlendError::DecodeArrayTooLarge {
		count: usize::MAX,
		max: opt.max_array_elems,
	})?;
	if count > opt.max_array_elems {
		return Err(BlendError::DecodeArrayTooLarge {
			count,
			max: opt.max_array_elems,
		});
	}

	let need = struct_size.checked_mul(count).ok_or(BlendError::DecodeArrayTooLarge {
		count,
		max: opt.max_array_elems,
	})?;
	if need > block.payload.len() {
		return Err(BlendError::DecodePayloadTooSmall {
			need,
			have: block.payload.len(),
		});
	}

	let mut cursor = Cursor::new(block.payload);
	let mut values = Vec::with_capacity(count);
	for _ in 0..count {
		let bytes = cursor.read_exact(struct_size)?;
		let value = decode_struct_impl(dna, sdna_nr, bytes, opt, 0)?;
		values.push(Value::Struct(value));
	}

	if count == 1 {
		Ok(values.pop().unwrap_or(Value::Null))
	} else {
		Ok(Value::Array(values))
	}
}

pub fn decode_struct_instance(dna: &Dna, sdna_nr: u32, bytes: &[u8], opt: &DecodeOptions) -> Result<StructValue> {
	decode_struct_impl(dna, sdna_nr, bytes, opt, 0)
}

fn decode_struct_impl(dna: &Dna, sdna_nr: u32, bytes: &[u8], opt: &DecodeOptions, depth: u32) -> Result<StructValue> {
	if depth >= opt.max_depth {
		return Err(BlendError::DecodeDepthExceeded { max_depth: opt.max_depth });
	}

	let item = dna.struct_by_sdna(sdna_nr).ok_or(BlendError::DecodeMissingSdna { sdna_nr })?;

	let mut cursor = Cursor::new(bytes);
	let mut fields = Vec::with_capacity(item.fields.len());

	for field in &item.fields {
		let type_name = dna.type_name(field.type_idx);
		let name_raw = dna.field_name(field.name_idx);
		let decl = parse_field_decl(name_raw);

		if !opt.include_padding && is_padding_field(decl.ident, type_name, decl.inline_array) {
			skip_field_storage(&mut cursor, dna, type_name, field.type_idx, &decl)?;
			continue;
		}

		let value = decode_field_value(&mut cursor, dna, field.type_idx, type_name, &decl, opt, depth + 1)?;
		fields.push(FieldValue {
			name: decl.ident.to_owned().into_boxed_str(),
			value,
		});
	}

	let type_name = dna.type_name(item.type_idx).to_owned();
	if cursor.remaining() > 0 {
		let leftover = cursor.remaining();
		if opt.strict_layout {
			return Err(BlendError::DecodeLayoutMismatch { type_name, leftover });
		}
		let _ = cursor.read_exact(leftover)?;
	}

	Ok(StructValue {
		type_name: type_name.into_boxed_str(),
		fields,
	})
}

fn decode_field_value(
	cursor: &mut Cursor<'_>,
	dna: &Dna,
	field_type_idx: u16,
	type_name: &str,
	decl: &FieldDecl<'_>,
	opt: &DecodeOptions,
	depth: u32,
) -> Result<Value> {
	let element_count = decl.inline_array;
	if element_count == 0 {
		return Ok(Value::Array(Vec::new()));
	}
	if element_count > opt.max_array_elems {
		return Err(BlendError::DecodeArrayTooLarge {
			count: element_count,
			max: opt.max_array_elems,
		});
	}

	if decl.ptr_depth > 0 || decl.is_func_ptr {
		return decode_pointer_values(cursor, element_count);
	}

	if let Some(sdna_idx) = dna.struct_for_type.get(field_type_idx as usize).and_then(|value| *value) {
		let size = usize::from(dna.tlen[field_type_idx as usize]);
		if size == 0 {
			return Ok(Value::Null);
		}

		let mut out = Vec::with_capacity(element_count);
		for _ in 0..element_count {
			let bytes = cursor.read_exact(size)?;
			let nested = decode_struct_impl(dna, sdna_idx, bytes, opt, depth)?;
			out.push(Value::Struct(nested));
		}

		if element_count == 1 {
			return Ok(out.pop().unwrap_or(Value::Null));
		}
		return Ok(Value::Array(out));
	}

	if opt.decode_char_arrays_as_string && type_name == "char" && element_count > 1 {
		let bytes = cursor.read_exact(element_count)?;
		let end = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
		return Ok(Value::String(String::from_utf8_lossy(&bytes[..end]).into_owned().into_boxed_str()));
	}

	decode_primitive_values(cursor, type_name, usize::from(dna.tlen[field_type_idx as usize]), element_count)
}

fn decode_pointer_values(cursor: &mut Cursor<'_>, count: usize) -> Result<Value> {
	let mut values = Vec::with_capacity(count);
	for _ in 0..count {
		let value = cursor.read_u64_le()?;
		values.push(Value::Ptr(value));
	}
	if count == 1 {
		Ok(values.pop().unwrap_or(Value::Null))
	} else {
		Ok(Value::Array(values))
	}
}

fn decode_primitive_values(cursor: &mut Cursor<'_>, type_name: &str, element_size: usize, count: usize) -> Result<Value> {
	let mut values = Vec::with_capacity(count);
	for _ in 0..count {
		let bytes = cursor.read_exact(element_size)?;
		values.push(decode_primitive(type_name, bytes));
	}

	if count == 1 {
		Ok(values.pop().unwrap_or(Value::Null))
	} else {
		Ok(Value::Array(values))
	}
}

fn decode_primitive(type_name: &str, bytes: &[u8]) -> Value {
	match (type_name, bytes.len()) {
		("float", 4) => {
			let mut arr = [0_u8; 4];
			arr.copy_from_slice(bytes);
			Value::F32(f32::from_le_bytes(arr))
		}
		("double", 8) => {
			let mut arr = [0_u8; 8];
			arr.copy_from_slice(bytes);
			Value::F64(f64::from_le_bytes(arr))
		}
		("bool", 1) => Value::Bool(bytes[0] != 0),
		(_, 1) => decode_int_i64_or_u64(type_name, u64::from(bytes[0]), 8),
		(_, 2) => {
			let mut arr = [0_u8; 2];
			arr.copy_from_slice(bytes);
			decode_int_i64_or_u64(type_name, u64::from(u16::from_le_bytes(arr)), 16)
		}
		(_, 4) => {
			let mut arr = [0_u8; 4];
			arr.copy_from_slice(bytes);
			decode_int_i64_or_u64(type_name, u64::from(u32::from_le_bytes(arr)), 32)
		}
		(_, 8) => {
			let mut arr = [0_u8; 8];
			arr.copy_from_slice(bytes);
			decode_int_i64_or_u64(type_name, u64::from_le_bytes(arr), 64)
		}
		_ => Value::Bytes(bytes.to_vec()),
	}
}

fn decode_int_i64_or_u64(type_name: &str, value: u64, bits: u32) -> Value {
	if is_unsigned_type(type_name) {
		return Value::U64(value);
	}

	let signed = match bits {
		8 => (value as i8) as i64,
		16 => (value as i16) as i64,
		32 => (value as i32) as i64,
		64 => value as i64,
		_ => value as i64,
	};
	Value::I64(signed)
}

fn is_unsigned_type(type_name: &str) -> bool {
	type_name.starts_with('u') || type_name.contains("uint") || type_name.contains("uchar")
}

fn skip_field_storage(cursor: &mut Cursor<'_>, dna: &Dna, type_name: &str, field_type_idx: u16, decl: &FieldDecl<'_>) -> Result<()> {
	let count = decl.inline_array;
	if count == 0 {
		return Ok(());
	}
	let element_size = if decl.ptr_depth > 0 || decl.is_func_ptr {
		POINTER_SIZE
	} else if type_name == "void" {
		1
	} else {
		let size = usize::from(dna.tlen[field_type_idx as usize]);
		if size == 0 { 1 } else { size }
	};
	let total = element_size.saturating_mul(count);
	let _ = cursor.read_exact(total)?;
	Ok(())
}

fn is_padding_field(ident: &str, type_name: &str, inline_array: usize) -> bool {
	(ident.starts_with("_pad") || ident.starts_with("pad")) && inline_array > 0 && matches!(type_name, "char" | "uchar" | "uint8_t")
}

#[derive(Debug, Clone, Copy)]
struct FieldDecl<'a> {
	ident: &'a str,
	ptr_depth: u8,
	inline_array: usize,
	is_func_ptr: bool,
	is_paren_ptr: bool,
}

fn parse_field_decl(raw: &str) -> FieldDecl<'_> {
	let trimmed = raw.trim();
	let mut decl = FieldDecl {
		ident: trimmed,
		ptr_depth: 0,
		inline_array: 1,
		is_func_ptr: trimmed.contains(")("),
		is_paren_ptr: false,
	};

	if let Some(start) = trimmed.find("(*") {
		let after = &trimmed[start + 2..];
		if let Some(close_idx) = after.find(')') {
			let inside = &after[..close_idx];
			let stars = inside.chars().take_while(|c| *c == '*').count();
			decl.ptr_depth = (stars as u8).saturating_add(1);
			let ident = inside.trim_start_matches('*').trim();
			if !ident.is_empty() {
				decl.ident = ident;
			}
			decl.is_paren_ptr = true;
			decl.inline_array = 1;
			return decl;
		}
	}

	let stars = trimmed.chars().take_while(|c| *c == '*').count();
	decl.ptr_depth = stars as u8;
	let mut tail = &trimmed[stars..];

	let ident_end = tail.find('[').unwrap_or(tail.len());
	let ident = tail[..ident_end].trim();
	if !ident.is_empty() {
		decl.ident = ident;
	}

	tail = &tail[ident_end..];
	if !decl.is_paren_ptr && !decl.is_func_ptr {
		let mut total = 1_usize;
		while let Some(start) = tail.find('[') {
			let Some(end) = tail[start + 1..].find(']') else {
				break;
			};
			let end = start + 1 + end;
			let dim = tail[start + 1..end].trim().parse::<usize>().unwrap_or(1);
			total = total.saturating_mul(dim);
			tail = &tail[end + 1..];
		}
		decl.inline_array = total;
	}

	decl
}

#[cfg(test)]
mod tests {
	use super::parse_field_decl;

	#[test]
	fn paren_pointer_depth_is_detected() {
		let decl = parse_field_decl("(*next)");
		assert_eq!(decl.ptr_depth, 1);
		assert_eq!(decl.ident, "next");
		assert_eq!(decl.inline_array, 1);
	}

	#[test]
	fn double_paren_pointer_depth_is_detected() {
		let decl = parse_field_decl("(**func)");
		assert_eq!(decl.ptr_depth, 2);
		assert_eq!(decl.ident, "func");
	}

	#[test]
	fn zero_sized_array_is_preserved() {
		let decl = parse_field_decl("weights[0]");
		assert_eq!(decl.inline_array, 0);
	}
}
