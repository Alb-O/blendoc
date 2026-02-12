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
	let from = index.canonicalize_ptr(dna, from_ptr)?;
	let to = index.canonicalize_ptr(dna, to_ptr)?;

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
mod tests;
