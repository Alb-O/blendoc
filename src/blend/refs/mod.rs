use std::sync::Arc;

use crate::blend::bytes::Cursor;
use crate::blend::decl::parse_field_decl;
use crate::blend::{BlendError, Dna, IdIndex, PointerIndex, Result};

/// Runtime limits for pointer-reference scanning.
#[derive(Debug, Clone, Copy)]
pub struct RefScanOptions {
	/// Maximum nested inline-struct expansion depth.
	pub max_depth: u32,
	/// Maximum supported inline array elements per field.
	pub max_array_elems: usize,
}

impl Default for RefScanOptions {
	fn default() -> Self {
		Self {
			max_depth: 1,
			max_array_elems: 4096,
		}
	}
}

/// One discovered pointer field reference from a scanned owner struct.
#[derive(Debug, Clone)]
pub struct RefRecord {
	/// Canonical pointer for the owner struct instance.
	pub owner_canonical: u64,
	/// Owner struct type name.
	pub owner_type: Arc<str>,
	/// Field path (`field` or `field.sub[i]`) where pointer was found.
	pub field: Arc<str>,
	/// Raw pointer value from struct bytes.
	pub ptr: u64,
	/// Resolution metadata when pointer maps to a known struct element.
	pub resolved: Option<RefTarget>,
}

/// Resolution metadata for one pointer target.
#[derive(Debug, Clone)]
pub struct RefTarget {
	/// Canonical pointer for resolved target element.
	pub canonical: u64,
	/// Block code containing the target.
	pub code: [u8; 4],
	/// SDNA index for resolved target type.
	pub sdna_nr: u32,
	/// Resolved target struct type name.
	pub type_name: Arc<str>,
	/// Optional ID name annotation when target is an ID-root block.
	pub id_name: Option<Arc<str>>,
}

/// Scan pointer fields from a resolved struct pointer.
pub fn scan_refs_from_ptr<'a>(dna: &Dna, index: &PointerIndex<'a>, id_index: &IdIndex, root_ptr: u64, options: &RefScanOptions) -> Result<Vec<RefRecord>> {
	let (owner_canonical, typed) = index.resolve_canonical_typed(dna, root_ptr)?;
	let element_index = typed.element_index.ok_or(BlendError::ChasePtrOutOfBounds { ptr: root_ptr })?;

	let owner_sdna = typed.base.entry.block.head.sdna_nr;
	let owner_struct = dna.struct_by_sdna(owner_sdna).ok_or(BlendError::DecodeMissingSdna { sdna_nr: owner_sdna })?;
	let owner_type = Arc::<str>::from(dna.type_name(owner_struct.type_idx));

	let owner_offset = element_index.checked_mul(typed.struct_size).ok_or(BlendError::ChaseSliceOob {
		start: usize::MAX,
		size: typed.struct_size,
		payload: typed.base.payload().len(),
	})?;
	let owner_end = owner_offset.checked_add(typed.struct_size).ok_or(BlendError::ChaseSliceOob {
		start: owner_offset,
		size: typed.struct_size,
		payload: typed.base.payload().len(),
	})?;
	let owner_bytes = typed.base.payload().get(owner_offset..owner_end).ok_or(BlendError::ChaseSliceOob {
		start: owner_offset,
		size: typed.struct_size,
		payload: typed.base.payload().len(),
	})?;

	let mut out = Vec::new();
	let mut scanner = RefScanner {
		dna,
		index,
		id_index,
		options,
		owner_canonical,
		owner_type,
		out: &mut out,
	};

	scanner.scan_struct(owner_sdna, owner_bytes, "", options.max_depth)?;
	Ok(out)
}

struct RefScanner<'a, 'b, 'c> {
	dna: &'a Dna,
	index: &'a PointerIndex<'a>,
	id_index: &'b IdIndex,
	options: &'a RefScanOptions,
	owner_canonical: u64,
	owner_type: Arc<str>,
	out: &'c mut Vec<RefRecord>,
}

impl<'a, 'b, 'c> RefScanner<'a, 'b, 'c> {
	fn scan_struct(&mut self, sdna_nr: u32, bytes: &[u8], prefix: &str, depth_left: u32) -> Result<()> {
		let item = self.dna.struct_by_sdna(sdna_nr).ok_or(BlendError::DecodeMissingSdna { sdna_nr })?;
		let mut cursor = Cursor::new(bytes);

		for field in &item.fields {
			let type_name = self.dna.type_name(field.type_idx);
			let decl = parse_field_decl(self.dna.field_name(field.name_idx));
			let count = decl.inline_array;

			if count > self.options.max_array_elems {
				return Err(BlendError::DecodeArrayTooLarge {
					count,
					max: self.options.max_array_elems,
				});
			}
			if count == 0 {
				continue;
			}

			if decl.ptr_depth > 0 || decl.is_func_ptr {
				for idx in 0..count {
					let ptr = cursor.read_u64_le()?;
					let field_name = if count == 1 {
						format!("{prefix}{}", decl.ident)
					} else {
						format!("{prefix}{}[{idx}]", decl.ident)
					};
					self.out.push(RefRecord {
						owner_canonical: self.owner_canonical,
						owner_type: self.owner_type.clone(),
						field: Arc::<str>::from(field_name),
						ptr,
						resolved: self.resolve_target(ptr),
					});
				}
				continue;
			}

			let element_size = if type_name == "void" {
				1
			} else {
				let size = usize::from(self.dna.tlen[field.type_idx as usize]);
				if size == 0 { 1 } else { size }
			};

			let nested_sdna = self.dna.struct_for_type.get(field.type_idx as usize).and_then(|value| *value);
			if let Some(nested_sdna) = nested_sdna
				&& depth_left > 0
				&& count == 1
			{
				let nested_bytes = cursor.read_exact(element_size)?;
				let next_prefix = format!("{prefix}{}.", decl.ident);
				self.scan_struct(nested_sdna, nested_bytes, &next_prefix, depth_left - 1)?;
				continue;
			}

			let total = element_size.saturating_mul(count);
			let _ = cursor.read_exact(total)?;
		}

		Ok(())
	}

	fn resolve_target(&self, ptr: u64) -> Option<RefTarget> {
		if ptr == 0 {
			return None;
		}

		let typed = self.index.resolve_typed(self.dna, ptr)?;
		let element_index = typed.element_index?;
		let offset = element_index.checked_mul(typed.struct_size)?;
		let offset = u64::try_from(offset).ok()?;
		let canonical = typed.base.entry.start_old.checked_add(offset)?;

		let type_name = Arc::<str>::from(
			self.dna
				.struct_by_sdna(typed.base.entry.block.head.sdna_nr)
				.map(|item| self.dna.type_name(item.type_idx))
				.unwrap_or("<unknown>"),
		);

		let id_name = self.id_index.get_by_ptr(canonical).map(|item| Arc::<str>::from(item.id_name.as_ref()));

		Some(RefTarget {
			canonical,
			code: typed.base.entry.block.head.code,
			sdna_nr: typed.base.entry.block.head.sdna_nr,
			type_name,
			id_name,
		})
	}
}

#[cfg(test)]
mod tests;
