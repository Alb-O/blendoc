use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::blend::{BlendError, Dna, IdIndex, PointerIndex, RefScanOptions, Result, scan_refs_from_ptr};

/// Runtime limits for shortest-route traversal.
#[derive(Debug, Clone)]
pub struct RouteOptions {
	/// Maximum BFS expansion depth.
	pub max_depth: u32,
	/// Maximum number of visited nodes.
	pub max_nodes: usize,
	/// Maximum number of explored edges.
	pub max_edges: usize,
	/// Per-node reference scan behavior.
	pub ref_scan: RefScanOptions,
}

impl Default for RouteOptions {
	fn default() -> Self {
		Self {
			max_depth: 6,
			max_nodes: 20_000,
			max_edges: 100_000,
			ref_scan: RefScanOptions::default(),
		}
	}
}

/// Reason route search stopped before exhausting graph.
#[derive(Debug, Clone, Copy)]
pub enum RouteTruncation {
	/// Search reached the configured depth ceiling.
	MaxDepth,
	/// Search reached the configured node budget.
	MaxNodes,
	/// Search reached the configured edge budget.
	MaxEdges,
}

/// One edge in a reconstructed route.
#[derive(Debug, Clone)]
pub struct RouteEdge {
	/// Source canonical pointer.
	pub from: u64,
	/// Target canonical pointer.
	pub to: u64,
	/// Source field path that produced this hop.
	pub field: Arc<str>,
}

/// Result of a route search between two pointers.
#[derive(Debug, Clone)]
pub struct RouteResult {
	/// Reconstructed shortest path when found.
	pub path: Option<Vec<RouteEdge>>,
	/// Number of unique visited nodes.
	pub visited_nodes: usize,
	/// Number of explored resolved edges.
	pub visited_edges: usize,
	/// Optional truncation reason when budgets stopped search.
	pub truncated: Option<RouteTruncation>,
}

/// Find a shortest pointer route between two pointers.
pub fn find_route_between_ptrs<'a>(
	dna: &Dna,
	index: &PointerIndex<'a>,
	ids: &IdIndex,
	from_ptr: u64,
	to_ptr: u64,
	options: &RouteOptions,
) -> Result<RouteResult> {
	let from = canonicalize_ptr(dna, index, from_ptr)?;
	let to = canonicalize_ptr(dna, index, to_ptr)?;

	if from == to {
		return Ok(RouteResult {
			path: Some(Vec::new()),
			visited_nodes: 1,
			visited_edges: 0,
			truncated: None,
		});
	}

	let mut queue = VecDeque::new();
	queue.push_back((from, 0_u32));

	let mut visited = HashSet::new();
	visited.insert(from);

	let mut parents: HashMap<u64, (u64, Arc<str>)> = HashMap::new();
	let mut visited_edges = 0_usize;
	let mut truncated = None;
	let mut hit_depth_limit = false;

	'outer: while let Some((current, depth)) = queue.pop_front() {
		if depth >= options.max_depth {
			hit_depth_limit = true;
			continue;
		}

		let refs = scan_refs_from_ptr(dna, index, ids, current, &options.ref_scan)?;
		let mut next_edges = Vec::new();
		for record in refs {
			let Some(target) = record.resolved else {
				continue;
			};

			visited_edges += 1;
			if visited_edges > options.max_edges {
				truncated = Some(RouteTruncation::MaxEdges);
				break 'outer;
			}

			next_edges.push((target.canonical, record.field));
		}

		next_edges.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

		for (next, via_field) in next_edges {
			if visited.contains(&next) {
				continue;
			}

			if visited.len() >= options.max_nodes {
				truncated = Some(RouteTruncation::MaxNodes);
				break 'outer;
			}

			visited.insert(next);
			parents.insert(next, (current, via_field.clone()));

			if next == to {
				let path = reconstruct_route(from, to, &parents)?;
				return Ok(RouteResult {
					path: Some(path),
					visited_nodes: visited.len(),
					visited_edges,
					truncated,
				});
			}

			queue.push_back((next, depth + 1));
		}
	}

	if truncated.is_none() && hit_depth_limit {
		truncated = Some(RouteTruncation::MaxDepth);
	}

	Ok(RouteResult {
		path: None,
		visited_nodes: visited.len(),
		visited_edges,
		truncated,
	})
}

fn canonicalize_ptr<'a>(dna: &Dna, index: &PointerIndex<'a>, ptr: u64) -> Result<u64> {
	if ptr == 0 {
		return Err(BlendError::ChaseNullPtr);
	}

	let typed = index.resolve_typed(dna, ptr).ok_or(BlendError::ChaseUnresolvedPtr { ptr })?;
	if typed.element_index.is_none() {
		return Err(BlendError::ChasePtrOutOfBounds { ptr });
	}
	index.canonical_ptr(dna, ptr).ok_or(BlendError::ChasePtrOutOfBounds { ptr })
}

fn reconstruct_route(from: u64, to: u64, parents: &HashMap<u64, (u64, Arc<str>)>) -> Result<Vec<RouteEdge>> {
	let mut out = Vec::new();
	let mut current = to;

	while current != from {
		let Some((prev, field)) = parents.get(&current) else {
			return Err(BlendError::ChaseUnresolvedPtr { ptr: current });
		};
		out.push(RouteEdge {
			from: *prev,
			to: current,
			field: field.clone(),
		});
		current = *prev;
	}

	out.reverse();
	Ok(out)
}

#[cfg(test)]
mod tests {
	use crate::blend::{
		BHead, Block, Dna, DnaField, DnaStruct, IdIndex, IdRecord, PointerIndex, PtrEntry, RefScanOptions, RouteOptions, find_route_between_ptrs,
	};

	#[test]
	fn finds_two_hop_route_in_synthetic_chain() {
		let payload_a = 0x2000_u64.to_le_bytes();
		let payload_b = 0x3000_u64.to_le_bytes();
		let payload_c = 0_u64.to_le_bytes();

		let block_a = Block {
			head: BHead {
				code: *b"DATA",
				sdna_nr: 0,
				old: 0x1000,
				len: 8,
				nr: 1,
			},
			payload: &payload_a,
			file_offset: 0,
		};
		let block_b = Block {
			head: BHead {
				code: *b"DATA",
				sdna_nr: 0,
				old: 0x2000,
				len: 8,
				nr: 1,
			},
			payload: &payload_b,
			file_offset: 32,
		};
		let block_c = Block {
			head: BHead {
				code: *b"DATA",
				sdna_nr: 0,
				old: 0x3000,
				len: 8,
				nr: 1,
			},
			payload: &payload_c,
			file_offset: 64,
		};

		let index = PointerIndex::from_entries_for_test(vec![
			PtrEntry {
				start_old: 0x1000,
				end_old: 0x1008,
				block: block_a,
			},
			PtrEntry {
				start_old: 0x2000,
				end_old: 0x2008,
				block: block_b,
			},
			PtrEntry {
				start_old: 0x3000,
				end_old: 0x3008,
				block: block_c,
			},
		]);

		let dna = Dna {
			names: vec!["*next".into()],
			types: vec!["Node".into()],
			tlen: vec![8],
			structs: vec![DnaStruct {
				type_idx: 0,
				fields: vec![DnaField { type_idx: 0, name_idx: 0 }],
			}],
			struct_for_type: vec![Some(0)],
		};

		let ids = IdIndex::build(vec![
			IdRecord {
				old_ptr: 0x1000,
				code: *b"A\0\0\0",
				sdna_nr: 0,
				type_name: "Node".into(),
				id_name: "AA".into(),
				next: None,
				prev: None,
				lib: None,
			},
			IdRecord {
				old_ptr: 0x2000,
				code: *b"B\0\0\0",
				sdna_nr: 0,
				type_name: "Node".into(),
				id_name: "BB".into(),
				next: None,
				prev: None,
				lib: None,
			},
			IdRecord {
				old_ptr: 0x3000,
				code: *b"C\0\0\0",
				sdna_nr: 0,
				type_name: "Node".into(),
				id_name: "CC".into(),
				next: None,
				prev: None,
				lib: None,
			},
		]);

		let result = find_route_between_ptrs(
			&dna,
			&index,
			&ids,
			0x1000,
			0x3000,
			&RouteOptions {
				max_depth: 3,
				max_nodes: 64,
				max_edges: 64,
				ref_scan: RefScanOptions {
					max_depth: 0,
					max_array_elems: 64,
				},
			},
		)
		.expect("route succeeds");

		let path = result.path.expect("path should be found");
		assert_eq!(path.len(), 2);
		assert_eq!(path[0].from, 0x1000);
		assert_eq!(path[0].to, 0x2000);
		assert_eq!(path[0].field.as_ref(), "next");
		assert_eq!(path[1].from, 0x2000);
		assert_eq!(path[1].to, 0x3000);
		assert_eq!(path[1].field.as_ref(), "next");
	}
}
