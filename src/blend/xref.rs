use std::sync::Arc;

use crate::blend::{BlendError, Dna, IdIndex, PointerIndex, RefScanOptions, Result, scan_refs_from_ptr};

/// One inbound reference into a target canonical pointer.
#[derive(Debug, Clone)]
pub struct InboundRef {
	/// Canonical owner pointer.
	pub from: u64,
	/// Owner type name.
	pub from_type: Arc<str>,
	/// Owner ID name when available.
	pub from_id: Option<Arc<str>>,
	/// Field path on the owner that points to the target.
	pub field: Arc<str>,
}

/// Configuration for inbound reference queries.
#[derive(Debug, Clone)]
pub struct XrefOptions {
	/// Nested field scan options for each owner.
	pub ref_scan: RefScanOptions,
	/// Maximum number of returned inbound matches.
	pub max_results: usize,
	/// Include unresolved refs that match the raw target pointer.
	pub include_unresolved: bool,
}

impl Default for XrefOptions {
	fn default() -> Self {
		Self {
			ref_scan: RefScanOptions::default(),
			max_results: 1024,
			include_unresolved: false,
		}
	}
}

/// Find inbound references to a canonicalized target pointer.
pub fn find_inbound_refs_to_ptr<'a>(dna: &Dna, index: &PointerIndex<'a>, ids: &IdIndex, target_ptr: u64, options: &XrefOptions) -> Result<Vec<InboundRef>> {
	if target_ptr == 0 {
		return Err(BlendError::ChaseNullPtr);
	}

	let target_canonical = canonical_ptr_for_target(dna, index, target_ptr)?;

	let mut out = Vec::new();
	for owner in &ids.records {
		let refs = scan_refs_from_ptr(dna, index, ids, owner.old_ptr, &options.ref_scan)?;
		for record in refs {
			let matches = match &record.resolved {
				Some(target) => target.canonical == target_canonical,
				None => options.include_unresolved && record.ptr == target_ptr,
			};
			if !matches {
				continue;
			}

			out.push(InboundRef {
				from: owner.old_ptr,
				from_type: Arc::<str>::from(owner.type_name.as_ref()),
				from_id: Some(Arc::<str>::from(owner.id_name.as_ref())),
				field: record.field,
			});

			if out.len() >= options.max_results {
				out.sort_by(|left, right| left.from.cmp(&right.from).then_with(|| left.field.cmp(&right.field)));
				return Ok(out);
			}
		}
	}

	out.sort_by(|left, right| left.from.cmp(&right.from).then_with(|| left.field.cmp(&right.field)));
	Ok(out)
}

fn canonical_ptr_for_target<'a>(dna: &Dna, index: &PointerIndex<'a>, ptr: u64) -> Result<u64> {
	let typed = index.resolve_typed(dna, ptr).ok_or(BlendError::ChaseUnresolvedPtr { ptr })?;
	let element_index = typed.element_index.ok_or(BlendError::ChasePtrOutOfBounds { ptr })?;
	let offset = element_index.checked_mul(typed.struct_size).ok_or(BlendError::ChasePtrOutOfBounds { ptr })?;
	let offset = u64::try_from(offset).map_err(|_| BlendError::ChasePtrOutOfBounds { ptr })?;
	typed.base.entry.start_old.checked_add(offset).ok_or(BlendError::ChasePtrOutOfBounds { ptr })
}

#[cfg(test)]
mod tests {
	use crate::blend::{
		BHead, Block, Dna, DnaField, DnaStruct, IdIndex, IdRecord, PointerIndex, PtrEntry, RefScanOptions, XrefOptions, find_inbound_refs_to_ptr,
	};

	#[test]
	fn nested_field_inbound_reference_is_reported() {
		let mut owner_payload = [0_u8; 16];
		owner_payload[8..16].copy_from_slice(&0x3000_u64.to_le_bytes());
		let target_payload = [0_u8; 8];

		let owner_block = Block {
			head: BHead {
				code: *b"SC\0\0",
				sdna_nr: 0,
				old: 0x1000,
				len: owner_payload.len() as u64,
				nr: 1,
			},
			payload: &owner_payload,
			file_offset: 0,
		};
		let target_block = Block {
			head: BHead {
				code: *b"WO\0\0",
				sdna_nr: 1,
				old: 0x3000,
				len: target_payload.len() as u64,
				nr: 1,
			},
			payload: &target_payload,
			file_offset: 64,
		};

		let index = PointerIndex::from_entries_for_test(vec![
			PtrEntry {
				start_old: 0x1000,
				end_old: 0x1010,
				block: owner_block,
			},
			PtrEntry {
				start_old: 0x3000,
				end_old: 0x3008,
				block: target_block,
			},
		]);

		let dna = Dna {
			names: vec!["id[8]".into(), "nested".into(), "*first".into()],
			types: vec!["char".into(), "Owner".into(), "Nested".into(), "Target".into()],
			tlen: vec![1, 16, 8, 8],
			structs: vec![
				DnaStruct {
					type_idx: 1,
					fields: vec![DnaField { type_idx: 0, name_idx: 0 }, DnaField { type_idx: 2, name_idx: 1 }],
				},
				DnaStruct {
					type_idx: 2,
					fields: vec![DnaField { type_idx: 3, name_idx: 2 }],
				},
				DnaStruct { type_idx: 3, fields: vec![] },
			],
			struct_for_type: vec![None, Some(0), Some(1), Some(2)],
		};

		let ids = IdIndex::build(vec![IdRecord {
			old_ptr: 0x1000,
			code: *b"SC\0\0",
			sdna_nr: 0,
			type_name: "Owner".into(),
			id_name: "SCOwner".into(),
			next: None,
			prev: None,
			lib: None,
		}]);

		let refs = find_inbound_refs_to_ptr(
			&dna,
			&index,
			&ids,
			0x3000,
			&XrefOptions {
				ref_scan: RefScanOptions {
					max_depth: 1,
					max_array_elems: 64,
				},
				max_results: 32,
				include_unresolved: false,
			},
		)
		.expect("xref succeeds");

		assert!(refs.iter().any(|item| item.from == 0x1000 && item.field.as_ref() == "nested.first"));
	}
}
