use std::collections::HashMap;

use crate::blend::decl::parse_field_decl;
use crate::blend::{BlendError, BlendFile, DecodeOptions, Dna, Result, StructValue, Value, decode_struct_instance};

/// One ID-root block summarized with extracted `ID` header fields.
#[derive(Debug, Clone)]
pub struct IdRecord {
	/// Block old-pointer base.
	pub old_ptr: u64,
	/// Source block code.
	pub code: [u8; 4],
	/// Source block SDNA index.
	pub sdna_nr: u32,
	/// Derived struct type name for the block SDNA index.
	pub type_name: Box<str>,
	/// Decoded `ID.name` field.
	pub id_name: Box<str>,
	/// Decoded `ID.next` pointer when present.
	pub next: Option<u64>,
	/// Decoded `ID.prev` pointer when present.
	pub prev: Option<u64>,
	/// Decoded `ID.lib` pointer when present.
	pub lib: Option<u64>,
}

/// Lookup index for ID records by canonical pointer and by `ID.name`.
#[derive(Debug, Clone)]
pub struct IdIndex {
	/// All scanned ID records.
	pub records: Vec<IdRecord>,
	by_ptr: HashMap<u64, usize>,
	by_name: HashMap<Box<str>, usize>,
}

impl IdIndex {
	/// Build index maps for scanned ID records.
	pub fn build(records: Vec<IdRecord>) -> Self {
		let mut by_ptr = HashMap::new();
		let mut by_name = HashMap::new();
		for (idx, record) in records.iter().enumerate() {
			by_ptr.entry(record.old_ptr).or_insert(idx);
			by_name.entry(record.id_name.clone()).or_insert(idx);
		}

		Self { records, by_ptr, by_name }
	}

	/// Look up an ID record by canonical pointer.
	pub fn get_by_ptr(&self, ptr: u64) -> Option<&IdRecord> {
		let idx = self.by_ptr.get(&ptr)?;
		self.records.get(*idx)
	}

	/// Look up an ID record by exact `ID.name`.
	pub fn get_by_name(&self, name: &str) -> Option<&IdRecord> {
		let idx = self.by_name.get(name)?;
		self.records.get(*idx)
	}
}

#[derive(Debug, Clone, Copy)]
struct IdLayout {
	id_sdna: u32,
	id_size: usize,
}

/// Scan all blocks and extract `ID` headers for ID-root structs.
pub fn scan_id_blocks(file: &BlendFile, dna: &Dna) -> Result<Vec<IdRecord>> {
	let layout = detect_id_layout(dna)?;
	let id_roots = id_root_flags(dna);

	let decode = DecodeOptions {
		include_padding: true,
		strict_layout: true,
		..DecodeOptions::default()
	};

	let mut out = Vec::new();
	for block in file.blocks() {
		let block = block?;
		let is_id_root = id_roots.get(block.head.sdna_nr as usize).copied().unwrap_or(false);
		if !is_id_root {
			continue;
		}

		if block.payload.len() < layout.id_size {
			return Err(BlendError::DecodePayloadTooSmall {
				need: layout.id_size,
				have: block.payload.len(),
			});
		}

		let id = decode_struct_instance(dna, layout.id_sdna, &block.payload[..layout.id_size], &decode)?;
		let type_name = dna
			.struct_by_sdna(block.head.sdna_nr)
			.map(|item| dna.type_name(item.type_idx))
			.unwrap_or("<unknown>")
			.to_owned()
			.into_boxed_str();

		out.push(IdRecord {
			old_ptr: block.head.old,
			code: block.head.code,
			sdna_nr: block.head.sdna_nr,
			type_name,
			id_name: extract_name_field(&id)?.into_boxed_str(),
			next: extract_ptr_field(&id, "next"),
			prev: extract_ptr_field(&id, "prev"),
			lib: extract_ptr_field(&id, "lib"),
		});
	}
	out.sort_by_key(|item| item.old_ptr);

	Ok(out)
}

fn detect_id_layout(dna: &Dna) -> Result<IdLayout> {
	let Some(id_type_idx) = dna.types.iter().position(|item| item.as_ref() == "ID") else {
		return Err(BlendError::DnaStructNotFound { name: "ID".to_owned() });
	};

	let Some(id_sdna) = dna.struct_for_type.get(id_type_idx).and_then(|item| *item) else {
		return Err(BlendError::DnaStructNotFound { name: "ID".to_owned() });
	};

	let id_size = usize::from(dna.tlen[id_type_idx]);
	Ok(IdLayout { id_sdna, id_size })
}

fn id_root_flags(dna: &Dna) -> Vec<bool> {
	let mut out = vec![false; dna.structs.len()];

	for (sdna_idx, item) in dna.structs.iter().enumerate() {
		let Some(first) = item.fields.first() else {
			continue;
		};

		if dna.type_name(first.type_idx) != "ID" {
			continue;
		}

		if parse_field_decl(dna.field_name(first.name_idx)).ident == "id" {
			out[sdna_idx] = true;
		}
	}

	out
}

fn extract_name_field(item: &StructValue) -> Result<String> {
	let field = item
		.fields
		.iter()
		.find(|candidate| candidate.name.as_ref() == "name")
		.ok_or(BlendError::IdMissingName)?;
	match &field.value {
		Value::String(value) => Ok(value.to_string()),
		other => Err(BlendError::IdInvalidNameType {
			got: value_kind(other).to_owned(),
		}),
	}
}

fn extract_ptr_field(item: &StructValue, name: &str) -> Option<u64> {
	let field = item.fields.iter().find(|candidate| candidate.name.as_ref() == name)?;
	match field.value {
		Value::Ptr(ptr) => Some(ptr),
		_ => None,
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

#[cfg(test)]
mod tests;
