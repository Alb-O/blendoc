use crate::blend::{BlendError, BlendFile, DecodeOptions, Dna, PointerIndex, Result, StructValue, Value, decode_block_instances, decode_struct_instance};

#[derive(Debug, Clone, Copy)]
pub struct ChaseMeta {
	pub ptr: u64,
	pub resolved_block_code: [u8; 4],
	pub sdna_nr: u32,
	pub element_index: usize,
	pub element_offset: usize,
	pub struct_size: usize,
	pub block_old: u64,
}

pub fn chase_ptr_to_struct<'a>(dna: &Dna, index: &PointerIndex<'a>, ptr: u64, opt: &DecodeOptions) -> Result<Option<(ChaseMeta, StructValue)>> {
	if ptr == 0 {
		return Ok(None);
	}

	let typed = index.resolve_typed(dna, ptr).ok_or(BlendError::ChaseUnresolvedPtr { ptr })?;
	let element_index = typed.element_index.ok_or(BlendError::ChasePtrOutOfBounds { ptr })?;

	let start = element_index.checked_mul(typed.struct_size).ok_or(BlendError::ChaseSliceOob {
		start: usize::MAX,
		size: typed.struct_size,
		payload: typed.base.payload().len(),
	})?;
	let end = start.checked_add(typed.struct_size).ok_or(BlendError::ChaseSliceOob {
		start,
		size: typed.struct_size,
		payload: typed.base.payload().len(),
	})?;

	let bytes = typed.base.payload().get(start..end).ok_or(BlendError::ChaseSliceOob {
		start,
		size: typed.struct_size,
		payload: typed.base.payload().len(),
	})?;

	let value = decode_struct_instance(dna, typed.base.entry.block.head.sdna_nr, bytes, opt)?;
	let meta = ChaseMeta {
		ptr,
		resolved_block_code: typed.base.entry.block.head.code,
		sdna_nr: typed.base.entry.block.head.sdna_nr,
		element_index,
		element_offset: typed.element_offset,
		struct_size: typed.struct_size,
		block_old: typed.base.entry.start_old,
	};

	Ok(Some((meta, value)))
}

pub fn chase_scene_camera<'a>(
	file: &'a BlendFile,
	dna: &Dna,
	index: &PointerIndex<'a>,
	scene_decode: &DecodeOptions,
	object_decode: &DecodeOptions,
) -> Result<Option<(ChaseMeta, StructValue)>> {
	let Some(scene_block) = file.find_first_block_by_code([b'S', b'C', 0, 0])? else {
		return Ok(None);
	};

	let scene_value = decode_block_instances(dna, &scene_block, scene_decode)?;
	let scene = as_single_struct(scene_value)?;
	let camera_ptr = find_ptr_field(&scene, "camera")?;

	let Some((meta, object)) = chase_ptr_to_struct(dna, index, camera_ptr, object_decode)? else {
		return Ok(None);
	};

	if object.type_name.as_ref() != "Object" {
		return Err(BlendError::ChaseTypeMismatch {
			expected: "Object",
			got: object.type_name.to_string(),
		});
	}

	Ok(Some((meta, object)))
}

fn as_single_struct(value: Value) -> Result<StructValue> {
	match value {
		Value::Struct(item) => Ok(item),
		Value::Array(items) => {
			let Some(first) = items.into_iter().next() else {
				return Err(BlendError::ChaseTypeMismatch {
					expected: "Struct",
					got: "Array(empty)".to_owned(),
				});
			};
			match first {
				Value::Struct(item) => Ok(item),
				other => Err(BlendError::ChaseTypeMismatch {
					expected: "Struct",
					got: value_kind(&other).to_owned(),
				}),
			}
		}
		other => Err(BlendError::ChaseTypeMismatch {
			expected: "Struct",
			got: value_kind(&other).to_owned(),
		}),
	}
}

fn find_ptr_field(item: &StructValue, field: &'static str) -> Result<u64> {
	let Some(found) = item.fields.iter().find(|candidate| candidate.name.as_ref() == field) else {
		return Err(BlendError::ChaseMissingField {
			struct_name: item.type_name.to_string(),
			field,
		});
	};

	match found.value {
		Value::Ptr(ptr) => Ok(ptr),
		_ => Err(BlendError::ChaseExpectedPtr {
			struct_name: item.type_name.to_string(),
			field,
		}),
	}
}

fn value_kind(value: &Value) -> &'static str {
	match value {
		Value::Null => "Null",
		Value::Bool(_) => "Bool",
		Value::I64(_) => "I64",
		Value::U64(_) => "U64",
		Value::F32(_) => "F32",
		Value::F64(_) => "F64",
		Value::Bytes(_) => "Bytes",
		Value::String(_) => "String",
		Value::Ptr(_) => "Ptr",
		Value::Array(_) => "Array",
		Value::Struct(_) => "Struct",
	}
}
