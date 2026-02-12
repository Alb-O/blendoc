use std::collections::HashSet;
use std::sync::Arc;

use crate::blend::{Dna, IdIndex, PointerIndex, RefScanOptions, Result, scan_refs_from_ptr};

/// Options for whole-file ID-to-ID graph extraction.
#[derive(Debug, Clone)]
pub struct IdGraphOptions {
	/// Nested struct-scan behavior used per owner node.
	pub ref_scan: RefScanOptions,
	/// Maximum number of emitted edges.
	pub max_edges: usize,
	/// Keep self-edges when source and target canonical pointers match.
	pub include_self: bool,
}

impl Default for IdGraphOptions {
	fn default() -> Self {
		Self {
			ref_scan: RefScanOptions {
				max_depth: 1,
				max_array_elems: 4096,
			},
			max_edges: 100_000,
			include_self: false,
		}
	}
}

/// Truncation reason for ID graph extraction.
#[derive(Debug, Clone, Copy)]
pub enum IdGraphTruncation {
	/// Edge budget was reached.
	MaxEdges,
}

/// One ID graph node.
#[derive(Debug, Clone)]
pub struct IdGraphNode {
	/// Canonical node pointer (ID-root block old pointer).
	pub canonical: u64,
	/// Block code for this ID root.
	pub code: [u8; 4],
	/// SDNA index for this ID root.
	pub sdna_nr: u32,
	/// Type name for this ID root.
	pub type_name: Arc<str>,
	/// ID name for this ID root.
	pub id_name: Arc<str>,
}

/// One directed ID-to-ID edge.
#[derive(Debug, Clone)]
pub struct IdGraphEdge {
	/// Source canonical pointer.
	pub from: u64,
	/// Target canonical pointer.
	pub to: u64,
	/// Source field path that holds the pointer.
	pub field: Arc<str>,
}

/// Full ID graph extraction result.
#[derive(Debug, Clone)]
pub struct IdGraphResult {
	/// Extracted ID nodes.
	pub nodes: Vec<IdGraphNode>,
	/// Extracted ID edges.
	pub edges: Vec<IdGraphEdge>,
	/// Optional truncation reason.
	pub truncated: Option<IdGraphTruncation>,
}

/// Build a whole-file graph over ID-root records and ID-to-ID pointer fields.
pub fn build_id_graph<'a>(dna: &Dna, index: &PointerIndex<'a>, ids: &IdIndex, options: &IdGraphOptions) -> Result<IdGraphResult> {
	let mut nodes: Vec<IdGraphNode> = ids
		.records
		.iter()
		.map(|item| IdGraphNode {
			canonical: item.old_ptr,
			code: item.code,
			sdna_nr: item.sdna_nr,
			type_name: Arc::<str>::from(item.type_name.as_ref()),
			id_name: Arc::<str>::from(item.id_name.as_ref()),
		})
		.collect();
	nodes.sort_by_key(|item| item.canonical);

	let mut edges = Vec::new();
	let mut seen = HashSet::new();
	let mut truncated = None;

	'outer: for owner in &ids.records {
		let refs = scan_refs_from_ptr(dna, index, ids, owner.old_ptr, &options.ref_scan)?;
		for record in refs {
			let Some(target) = record.resolved else {
				continue;
			};
			if target.id_name.is_none() {
				continue;
			}
			if !options.include_self && owner.old_ptr == target.canonical {
				continue;
			}

			let key = (owner.old_ptr, target.canonical, record.field.clone());
			if !seen.insert(key.clone()) {
				continue;
			}

			if edges.len() >= options.max_edges {
				truncated = Some(IdGraphTruncation::MaxEdges);
				break 'outer;
			}

			edges.push(IdGraphEdge {
				from: key.0,
				to: key.1,
				field: key.2,
			});
		}
	}

	edges.sort_by(|left, right| {
		left.from
			.cmp(&right.from)
			.then_with(|| left.to.cmp(&right.to))
			.then_with(|| left.field.cmp(&right.field))
	});

	Ok(IdGraphResult { nodes, edges, truncated })
}
