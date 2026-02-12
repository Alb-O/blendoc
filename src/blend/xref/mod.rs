use std::sync::Arc;

use crate::blend::{Dna, IdIndex, PointerIndex, RefScanOptions, Result, scan_refs_from_ptr};

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
	let target_canonical = index.canonicalize_ptr(dna, target_ptr)?;

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

#[cfg(test)]
mod tests;
