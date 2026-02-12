use std::collections::HashMap;
use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, IdIndex, RouteOptions, RouteResult, RouteTruncation, find_route_between_ptrs, scan_id_blocks};

use crate::cmd::util::{IdOrPtrSelector, RootSelector, emit_json, parse_id_or_ptr_selector, parse_root_selector, ptr_hex, render_code};

#[derive(clap::Args)]
pub struct Args {
	pub file: PathBuf,
	#[arg(long = "from-id")]
	pub from_id: Option<String>,
	#[arg(long = "from-ptr")]
	pub from_ptr: Option<String>,
	#[arg(long = "from-code")]
	pub from_code: Option<String>,
	#[arg(long = "to-id")]
	pub to_id: Option<String>,
	#[arg(long = "to-ptr")]
	pub to_ptr: Option<String>,
	#[arg(long)]
	pub depth: Option<u32>,
	#[arg(long = "refs-depth")]
	pub refs_depth: Option<u32>,
	#[arg(long = "max-nodes")]
	pub max_nodes: Option<usize>,
	#[arg(long = "max-edges")]
	pub max_edges: Option<usize>,
	#[arg(long)]
	pub json: bool,
}

/// Find and print a shortest pointer route between two endpoints.
pub fn run(args: Args) -> blendoc::blend::Result<()> {
	let Args {
		file: path,
		from_id,
		from_ptr,
		from_code,
		to_id,
		to_ptr,
		depth,
		refs_depth,
		max_nodes,
		max_edges,
		json,
	} = args;

	let from_selector = parse_root_selector(from_code, from_ptr, from_id)?;
	let to_selector = parse_id_or_ptr_selector(to_id, to_ptr)?;

	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;
	let index = blend.pointer_index()?;
	let ids = IdIndex::build(scan_id_blocks(&blend, &dna)?);

	let (from_ptr, from_label) = match from_selector {
		RootSelector::Id(name) => {
			let row = ids.get_by_name(&name).ok_or(BlendError::IdRecordNotFound { name: name.clone() })?;
			(row.old_ptr, format!("id:{}", row.id_name))
		}
		RootSelector::Ptr(ptr) => (ptr, format!("ptr:0x{ptr:016x}")),
		RootSelector::Code(code) => {
			let block = blend.find_first_block_by_code(code)?.ok_or(BlendError::BlockNotFound { code })?;
			(block.head.old, format!("code:{}", render_code(code)))
		}
	};

	let (to_ptr, to_label) = match to_selector {
		IdOrPtrSelector::Id(name) => {
			let row = ids.get_by_name(&name).ok_or(BlendError::IdRecordNotFound { name: name.clone() })?;
			(row.old_ptr, format!("id:{}", row.id_name))
		}
		IdOrPtrSelector::Ptr(ptr) => (ptr, format!("ptr:0x{ptr:016x}")),
	};

	let mut options = RouteOptions::default();
	if let Some(depth) = depth {
		options.max_depth = depth;
	}
	if let Some(refs_depth) = refs_depth {
		options.ref_scan.max_depth = refs_depth;
	}
	if let Some(max_nodes) = max_nodes {
		options.max_nodes = max_nodes;
	}
	if let Some(max_edges) = max_edges {
		options.max_edges = max_edges;
	}

	let result = find_route_between_ptrs(&dna, &index, &ids, from_ptr, to_ptr, &options)?;

	let from_meta = resolve_node_meta(&dna, &index, &ids, from_ptr)?;
	let to_meta = resolve_node_meta(&dna, &index, &ids, to_ptr)?;

	if json {
		print_json(&path, &from_label, &to_label, &from_meta, &to_meta, &result);
		return Ok(());
	}

	println!("path: {}", path.display());
	println!("from: {from_label}");
	println!("to: {to_label}");
	println!("from_canonical: 0x{:016x}", from_meta.canonical);
	println!("to_canonical: 0x{:016x}", to_meta.canonical);
	println!("visited_nodes: {}", result.visited_nodes);
	println!("visited_edges: {}", result.visited_edges);
	println!("truncated: {}", truncation_label_opt(result.truncated));

	let mut labels = HashMap::new();
	labels.insert(from_meta.canonical, from_meta.clone());
	labels.insert(to_meta.canonical, to_meta.clone());

	if let Some(path_edges) = &result.path {
		println!("route_len: {}", path_edges.len());
		for edge in path_edges {
			let from = if let Some(existing) = labels.get(&edge.from) {
				existing.clone()
			} else {
				let resolved = resolve_node_meta(&dna, &index, &ids, edge.from)?;
				labels.insert(edge.from, resolved.clone());
				resolved
			};
			let to = if let Some(existing) = labels.get(&edge.to) {
				existing.clone()
			} else {
				let resolved = resolve_node_meta(&dna, &index, &ids, edge.to)?;
				labels.insert(edge.to, resolved.clone());
				resolved
			};
			println!("{} -{}-> {}", node_label(&from), edge.field, node_label(&to));
		}
	} else {
		println!("route_len: not_found");
	}

	Ok(())
}

#[derive(Debug, Clone)]
struct NodeMeta {
	canonical: u64,
	type_name: String,
	id_name: Option<String>,
}

fn resolve_node_meta<'a>(dna: &blendoc::blend::Dna, index: &blendoc::blend::PointerIndex<'a>, ids: &IdIndex, ptr: u64) -> blendoc::blend::Result<NodeMeta> {
	let (canonical, typed) = index.resolve_canonical_typed(dna, ptr)?;

	let type_name = dna
		.struct_by_sdna(typed.base.entry.block.head.sdna_nr)
		.map(|item| dna.type_name(item.type_idx))
		.unwrap_or("<unknown>")
		.to_owned();

	Ok(NodeMeta {
		canonical,
		type_name,
		id_name: ids.get_by_ptr(canonical).map(|item| item.id_name.to_string()),
	})
}

fn node_label(node: &NodeMeta) -> String {
	if let Some(id_name) = &node.id_name {
		format!("{}({})", id_name, node.type_name)
	} else {
		format!("{}@0x{:016x}", node.type_name, node.canonical)
	}
}

fn truncation_label_opt(value: Option<RouteTruncation>) -> &'static str {
	match value {
		Some(RouteTruncation::MaxDepth) => "max_depth",
		Some(RouteTruncation::MaxNodes) => "max_nodes",
		Some(RouteTruncation::MaxEdges) => "max_edges",
		None => "none",
	}
}

fn truncation_label(value: RouteTruncation) -> &'static str {
	match value {
		RouteTruncation::MaxDepth => "max_depth",
		RouteTruncation::MaxNodes => "max_nodes",
		RouteTruncation::MaxEdges => "max_edges",
	}
}

fn print_json(path: &std::path::Path, from_label: &str, to_label: &str, from: &NodeMeta, to: &NodeMeta, result: &RouteResult) {
	let payload = RouteJson {
		path: path.display().to_string(),
		from: EndpointJson {
			selector: from_label.to_owned(),
			canonical: ptr_hex(from.canonical),
			type_name: from.type_name.clone(),
			id: from.id_name.clone(),
		},
		to: EndpointJson {
			selector: to_label.to_owned(),
			canonical: ptr_hex(to.canonical),
			type_name: to.type_name.clone(),
			id: to.id_name.clone(),
		},
		visited_nodes: result.visited_nodes,
		visited_edges: result.visited_edges,
		truncated: result.truncated.map(truncation_label).map(str::to_owned),
		path_edges: result
			.path
			.as_deref()
			.unwrap_or(&[])
			.iter()
			.map(|edge| RouteEdgeJson {
				from: ptr_hex(edge.from),
				to: ptr_hex(edge.to),
				field: edge.field.to_string(),
			})
			.collect(),
	};

	emit_json(&payload);
}

#[derive(serde::Serialize)]
struct EndpointJson {
	selector: String,
	canonical: String,
	#[serde(rename = "type")]
	type_name: String,
	id: Option<String>,
}

#[derive(serde::Serialize)]
struct RouteEdgeJson {
	from: String,
	to: String,
	field: String,
}

#[derive(serde::Serialize)]
struct RouteJson {
	path: String,
	from: EndpointJson,
	to: EndpointJson,
	visited_nodes: usize,
	visited_edges: usize,
	truncated: Option<String>,
	path_edges: Vec<RouteEdgeJson>,
}

#[cfg(test)]
mod tests;
