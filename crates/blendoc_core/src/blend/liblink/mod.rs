use std::sync::Arc;

use crate::blend::{
	BlendFile, DecodeOptions, Dna, IdIndex, RefRecord, RefScanOptions, Result, StructValue, Value, decode_ptr_instance, scan_id_blocks, scan_refs_from_ptr,
};

/// One declared external library record from a `Library` (`LI`) ID block.
#[derive(Debug, Clone)]
pub struct LibraryRecord {
	/// Canonical pointer of the `Library` ID record.
	pub id_ptr: u64,
	/// ID name (for example `LIsword.blend`).
	pub id_name: Arc<str>,
	/// Decoded `Library.name` path (for example `//sword.blend`).
	pub library_path: Arc<str>,
	/// Whether the path uses Blender-relative `//` notation.
	pub is_relative: bool,
}

/// Signal indicating why an ID is considered linked or library-related.
#[derive(Debug, Clone)]
pub enum LinkSignal {
	/// `ID.lib` pointer is non-null.
	IdLibPtr {
		/// Raw pointer value.
		ptr: u64,
	},
	/// `ID.override_library` pointer is non-null.
	OverrideLibraryPtr {
		/// Raw pointer value.
		ptr: u64,
	},
	/// `ID.library_weak_reference` pointer is non-null.
	LibraryWeakReferencePtr {
		/// Raw pointer value.
		ptr: u64,
	},
	/// The file declares a `Library` (`LI`) ID entry.
	LibraryIdPresent {
		/// `Library` ID name.
		library_id_name: Arc<str>,
		/// Decoded `Library.name` path.
		library_path: Arc<str>,
	},
}

/// Confidence level for linked-library provenance classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkConfidence {
	/// No link evidence found for this ID.
	None,
	/// Weak context-only evidence (for example this row is a `Library` ID declaration).
	Low,
	/// Medium evidence from override/weak-reference pointers.
	Medium,
	/// Strong evidence from a direct `ID.lib` pointer.
	High,
}

impl LinkConfidence {
	/// Stable machine label for JSON/text output.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Low => "low",
			Self::Medium => "medium",
			Self::High => "high",
		}
	}

	/// Monotonic ordering rank useful for comparisons in tests.
	pub fn rank(self) -> u8 {
		match self {
			Self::None => 0,
			Self::Low => 1,
			Self::Medium => 2,
			Self::High => 3,
		}
	}
}

/// Linked-library provenance metadata for one ID record.
#[derive(Debug, Clone)]
pub struct IdLinkProvenance {
	/// Canonical ID pointer.
	pub id_ptr: u64,
	/// ID name.
	pub id_name: Arc<str>,
	/// ID type name.
	pub type_name: Arc<str>,
	/// Whether the ID should be treated as linked/library-related.
	pub linked: bool,
	/// Confidence level for the classification.
	pub confidence: LinkConfidence,
	/// Evidence signals used for classification.
	pub signals: Vec<LinkSignal>,
}

/// Scan `Library` (`LI`) ID records and decode library file paths.
pub fn scan_library_records(file: &BlendFile, dna: &Dna) -> Result<Vec<LibraryRecord>> {
	let ids = scan_id_blocks(file, dna)?;
	let index = file.pointer_index()?;
	let decode = DecodeOptions {
		include_padding: true,
		strict_layout: true,
		..DecodeOptions::default()
	};

	let mut out = Vec::new();
	for item in ids {
		if !is_library_id(item.code, &item.type_name) {
			continue;
		}

		let (_, value) = decode_ptr_instance(dna, &index, item.old_ptr, &decode)?;
		let library_path = extract_struct_string_field(&value, "name").unwrap_or_else(|| Arc::<str>::from("<unknown>"));
		out.push(LibraryRecord {
			id_ptr: item.old_ptr,
			id_name: Arc::<str>::from(item.id_name.as_ref()),
			is_relative: library_path.starts_with("//"),
			library_path,
		});
	}

	out.sort_by_key(|item| item.id_ptr);
	Ok(out)
}

/// Scan per-ID linked-library provenance from ID header pointers and library declarations.
pub fn scan_id_link_provenance(file: &BlendFile, dna: &Dna) -> Result<Vec<IdLinkProvenance>> {
	let records = scan_id_blocks(file, dna)?;
	let libraries = scan_library_records(file, dna)?;
	let index = file.pointer_index()?;
	let id_index = IdIndex::build(records.clone());
	let ref_options = RefScanOptions {
		max_depth: 1,
		max_array_elems: 4096,
	};

	let mut out = Vec::new();
	for item in records {
		let refs = scan_refs_from_ptr(dna, &index, &id_index, item.old_ptr, &ref_options)?;
		let id_lib_ptr = item.lib.filter(|ptr| *ptr != 0).or_else(|| field_ptr(&refs, "id.lib"));
		let override_ptr = field_ptr(&refs, "id.override_library");
		let weak_ptr = field_ptr(&refs, "id.library_weak_reference");

		let mut signals = Vec::new();
		if let Some(ptr) = id_lib_ptr {
			signals.push(LinkSignal::IdLibPtr { ptr });
		}
		if let Some(ptr) = override_ptr {
			signals.push(LinkSignal::OverrideLibraryPtr { ptr });
		}
		if let Some(ptr) = weak_ptr {
			signals.push(LinkSignal::LibraryWeakReferencePtr { ptr });
		}

		if is_library_id(item.code, &item.type_name)
			&& let Some(record) = libraries.iter().find(|record| record.id_ptr == item.old_ptr)
		{
			signals.push(LinkSignal::LibraryIdPresent {
				library_id_name: record.id_name.clone(),
				library_path: record.library_path.clone(),
			});
		}

		let confidence = if id_lib_ptr.is_some() {
			LinkConfidence::High
		} else if override_ptr.is_some() || weak_ptr.is_some() {
			LinkConfidence::Medium
		} else if signals.iter().any(|signal| matches!(signal, LinkSignal::LibraryIdPresent { .. })) {
			LinkConfidence::Low
		} else {
			LinkConfidence::None
		};

		out.push(IdLinkProvenance {
			id_ptr: item.old_ptr,
			id_name: Arc::<str>::from(item.id_name.as_ref()),
			type_name: Arc::<str>::from(item.type_name.as_ref()),
			linked: !matches!(confidence, LinkConfidence::None),
			confidence,
			signals,
		});
	}

	out.sort_by_key(|item| item.id_ptr);
	Ok(out)
}

fn is_library_id(code: [u8; 4], type_name: &str) -> bool {
	code == [b'L', b'I', 0, 0] || type_name == "Library"
}

fn extract_struct_string_field(value: &StructValue, field_name: &str) -> Option<Arc<str>> {
	let field = value.fields.iter().find(|field| field.name.as_ref() == field_name)?;
	match &field.value {
		Value::String(value) => Some(Arc::<str>::from(value.as_ref())),
		_ => None,
	}
}

fn field_ptr(refs: &[RefRecord], field_name: &str) -> Option<u64> {
	refs.iter()
		.find(|item| item.field.as_ref() == field_name)
		.map(|item| item.ptr)
		.filter(|ptr| *ptr != 0)
}

#[cfg(test)]
mod tests;
