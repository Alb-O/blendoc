use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::blend::{BlendError, Dna, IdIndex, PointerIndex, RefScanOptions, Result, scan_refs_from_ptr};

/// Runtime limits and filters for pointer-graph extraction.
#[derive(Debug, Clone)]
pub struct GraphOptions {
	/// Maximum BFS depth to expand from the root node.
	pub max_depth: u32,
	/// Maximum number of discovered nodes before truncation.
	pub max_nodes: usize,
	/// Maximum number of discovered edges before truncation.
	pub max_edges: usize,
	/// Options passed through to pointer-reference scanning.
	pub ref_scan: RefScanOptions,
	/// Keep only nodes with ID names in final output.
	pub id_only: bool,
	/// Skip null pointers while collecting edges.
	pub skip_null_ptrs: bool,
}

impl Default for GraphOptions {
	fn default() -> Self {
		Self {
			max_depth: 2,
			max_nodes: 4096,
			max_edges: 16384,
			ref_scan: RefScanOptions::default(),
			id_only: false,
			skip_null_ptrs: true,
		}
	}
}

/// One graph node keyed by canonical pointer.
#[derive(Debug, Clone)]
pub struct GraphNode {
	/// Canonical pointer for this node.
	pub canonical: u64,
	/// Block code where the canonical pointer resolves.
	pub code: [u8; 4],
	/// SDNA index for resolved type.
	pub sdna_nr: u32,
	/// Resolved struct type name.
	pub type_name: Arc<str>,
	/// Optional `ID.name` annotation.
	pub id_name: Option<Arc<str>>,
}

/// Directed pointer edge between canonical nodes.
#[derive(Debug, Clone)]
pub struct GraphEdge {
	/// Source canonical pointer.
	pub from: u64,
	/// Target canonical pointer.
	pub to: u64,
	/// Source field path carrying this pointer.
	pub field: Arc<str>,
}

/// Truncation reason when extraction hits configured limits.
#[derive(Debug, Clone, Copy)]
pub enum GraphTruncation {
	/// Expansion stopped due depth ceiling.
	MaxDepth,
	/// Expansion stopped due node budget.
	MaxNodes,
	/// Expansion stopped due edge budget.
	MaxEdges,
}

/// Extracted shallow pointer graph.
#[derive(Debug, Clone)]
pub struct GraphResult {
	/// Final node set.
	pub nodes: Vec<GraphNode>,
	/// Final edge set.
	pub edges: Vec<GraphEdge>,
	/// Optional truncation marker.
	pub truncated: Option<GraphTruncation>,
}

/// Build a depth-limited pointer graph from a root pointer.
pub fn build_graph_from_ptr<'a>(dna: &Dna, index: &PointerIndex<'a>, ids: &IdIndex, root_ptr: u64, options: &GraphOptions) -> Result<GraphResult> {
	if root_ptr == 0 {
		return Err(BlendError::ChaseNullPtr);
	}

	let root = resolve_graph_node(dna, index, ids, root_ptr)?;
	let root_canonical = root.canonical;

	let mut nodes_by_ptr = HashMap::new();
	nodes_by_ptr.insert(root.canonical, root);

	let mut queue = VecDeque::new();
	queue.push_back((root_canonical, 0_u32));

	let mut seen = HashSet::new();
	seen.insert(root_canonical);

	let mut edges = Vec::new();
	let mut edge_seen: HashSet<(u64, u64, Arc<str>)> = HashSet::new();

	let mut truncated = None;
	let mut hit_depth_limit = false;

	'outer: while let Some((current, depth)) = queue.pop_front() {
		if depth >= options.max_depth {
			hit_depth_limit = true;
			continue;
		}

		let refs = scan_refs_from_ptr(dna, index, ids, current, &options.ref_scan)?;
		for record in refs {
			if options.skip_null_ptrs && record.ptr == 0 {
				continue;
			}

			let Some(target) = record.resolved else {
				continue;
			};

			if !nodes_by_ptr.contains_key(&target.canonical) {
				if nodes_by_ptr.len() >= options.max_nodes {
					truncated = Some(GraphTruncation::MaxNodes);
					break 'outer;
				}

				nodes_by_ptr.insert(
					target.canonical,
					GraphNode {
						canonical: target.canonical,
						code: target.code,
						sdna_nr: target.sdna_nr,
						type_name: target.type_name,
						id_name: target.id_name,
					},
				);
			}

			if seen.insert(target.canonical) {
				queue.push_back((target.canonical, depth + 1));
			}

			let edge_key = (current, target.canonical, record.field.clone());
			if edge_seen.insert(edge_key.clone()) {
				if edges.len() >= options.max_edges {
					truncated = Some(GraphTruncation::MaxEdges);
					break 'outer;
				}
				edges.push(GraphEdge {
					from: edge_key.0,
					to: edge_key.1,
					field: edge_key.2,
				});
			}
		}
	}

	if truncated.is_none() && hit_depth_limit {
		truncated = Some(GraphTruncation::MaxDepth);
	}

	let mut nodes: Vec<GraphNode> = nodes_by_ptr.into_values().collect();
	nodes.sort_by_key(|node| node.canonical);

	if options.id_only {
		let mut allowed = HashSet::new();
		for node in &nodes {
			if node.id_name.is_some() || node.canonical == root_canonical {
				allowed.insert(node.canonical);
			}
		}
		nodes.retain(|node| allowed.contains(&node.canonical));
		edges.retain(|edge| allowed.contains(&edge.from) && allowed.contains(&edge.to));
	}

	edges.sort_by(|a, b| a.from.cmp(&b.from).then_with(|| a.to.cmp(&b.to)).then_with(|| a.field.cmp(&b.field)));

	Ok(GraphResult { nodes, edges, truncated })
}

fn resolve_graph_node<'a>(dna: &Dna, index: &PointerIndex<'a>, ids: &IdIndex, ptr: u64) -> Result<GraphNode> {
	let (canonical, typed) = index.resolve_canonical_typed(dna, ptr)?;

	let type_name = dna
		.struct_by_sdna(typed.base.entry.block.head.sdna_nr)
		.map(|item| dna.type_name(item.type_idx))
		.unwrap_or("<unknown>");

	Ok(GraphNode {
		canonical,
		code: typed.base.entry.block.head.code,
		sdna_nr: typed.base.entry.block.head.sdna_nr,
		type_name: Arc::<str>::from(type_name),
		id_name: ids.get_by_ptr(canonical).map(|item| Arc::<str>::from(item.id_name.as_ref())),
	})
}

#[cfg(test)]
mod tests;
