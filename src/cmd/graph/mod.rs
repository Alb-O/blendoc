use std::collections::HashMap;
use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, GraphOptions, GraphResult, GraphTruncation, IdIndex, build_graph_from_ptr, scan_id_blocks};

use crate::cmd::util::{RootSelector, dot_escape, emit_json, parse_root_selector, ptr_hex, render_code};

#[derive(clap::Args)]
pub struct Args {
	pub file: PathBuf,
	#[arg(long)]
	pub code: Option<String>,
	#[arg(long)]
	pub ptr: Option<String>,
	#[arg(long = "id")]
	pub id_name: Option<String>,
	#[arg(long)]
	pub depth: Option<u32>,
	#[arg(long = "refs-depth")]
	pub refs_depth: Option<u32>,
	#[arg(long = "max-nodes")]
	pub max_nodes: Option<usize>,
	#[arg(long = "max-edges")]
	pub max_edges: Option<usize>,
	#[arg(long = "id-only")]
	pub id_only: bool,
	#[arg(long)]
	pub dot: bool,
	#[arg(long)]
	pub json: bool,
}

/// Build and print a shallow pointer graph from one root selector.
pub fn run(args: Args) -> blendoc::blend::Result<()> {
	let Args {
		file: path,
		code,
		ptr,
		id_name,
		depth,
		refs_depth,
		max_nodes,
		max_edges,
		id_only,
		dot,
		json,
	} = args;

	let selector = parse_root_selector(code, ptr, id_name)?;

	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;
	let index = blend.pointer_index()?;
	let ids = IdIndex::build(scan_id_blocks(&blend, &dna)?);

	let (root_ptr, root_label) = match selector {
		RootSelector::Code(block_code) => {
			let block = blend
				.find_first_block_by_code(block_code)?
				.ok_or(BlendError::BlockNotFound { code: block_code })?;
			(block.head.old, format!("code:{}", render_code(block_code)))
		}
		RootSelector::Ptr(ptr) => (ptr, format!("ptr:0x{ptr:016x}")),
		RootSelector::Id(name) => {
			let row = ids.get_by_name(&name).ok_or(BlendError::IdRecordNotFound { name: name.clone() })?;
			(row.old_ptr, format!("id:{}", row.id_name))
		}
	};

	let mut options = GraphOptions::default();
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
	options.id_only = id_only;

	let graph = build_graph_from_ptr(&dna, &index, &ids, root_ptr, &options)?;

	if json {
		print_json(&path, &root_label, root_ptr, &graph);
		return Ok(());
	}
	if dot {
		print_dot(&graph);
		return Ok(());
	}

	print_text(&path, &root_label, root_ptr, &graph);
	Ok(())
}

fn print_text(path: &std::path::Path, root_label: &str, root_ptr: u64, graph: &GraphResult) {
	println!("path: {}", path.display());
	println!("root: {root_label}");
	println!("root_ptr: 0x{root_ptr:016x}");
	println!("nodes: {}", graph.nodes.len());
	println!("edges: {}", graph.edges.len());
	println!("truncated: {}", truncation_label(graph.truncated));

	let by_ptr: HashMap<u64, _> = graph.nodes.iter().map(|node| (node.canonical, node)).collect();
	for edge in &graph.edges {
		let from = by_ptr.get(&edge.from).copied();
		let to = by_ptr.get(&edge.to).copied();
		println!("{} -{}-> {}", node_label(from), edge.field, node_label(to));
	}
}

fn print_dot(graph: &GraphResult) {
	println!("digraph blendoc {{");
	for node in &graph.nodes {
		let label = if let Some(id_name) = &node.id_name {
			format!("{}\\n{}", id_name, node.type_name)
		} else {
			format!("{}\\n0x{:016x}", node.type_name, node.canonical)
		};
		println!("  \"0x{:016x}\" [label=\"{}\"]", node.canonical, dot_escape(&label));
	}
	for edge in &graph.edges {
		println!("  \"0x{:016x}\" -> \"0x{:016x}\" [label=\"{}\"]", edge.from, edge.to, dot_escape(&edge.field));
	}
	println!("}}");
}

fn print_json(path: &std::path::Path, root_label: &str, root_ptr: u64, graph: &GraphResult) {
	let payload = GraphJson {
		path: path.display().to_string(),
		root: root_label.to_owned(),
		root_ptr: ptr_hex(root_ptr),
		truncated: truncation_value(graph.truncated).map(str::to_owned),
		nodes: graph
			.nodes
			.iter()
			.map(|node| GraphNodeJson {
				canonical: ptr_hex(node.canonical),
				code: render_code(node.code),
				sdna_nr: node.sdna_nr,
				type_name: node.type_name.to_string(),
				id: node.id_name.as_deref().map(|item| item.to_string()),
			})
			.collect(),
		edges: graph
			.edges
			.iter()
			.map(|edge| GraphEdgeJson {
				from: ptr_hex(edge.from),
				to: ptr_hex(edge.to),
				field: edge.field.to_string(),
			})
			.collect(),
	};

	emit_json(&payload);
}

fn node_label(node: Option<&blendoc::blend::GraphNode>) -> String {
	let Some(node) = node else {
		return "<unknown>".to_owned();
	};

	if let Some(id_name) = &node.id_name {
		format!("{}({})", id_name, node.type_name)
	} else {
		format!("{}@0x{:016x}", node.type_name, node.canonical)
	}
}

fn truncation_label(value: Option<GraphTruncation>) -> &'static str {
	match value {
		Some(GraphTruncation::MaxDepth) => "max_depth",
		Some(GraphTruncation::MaxNodes) => "max_nodes",
		Some(GraphTruncation::MaxEdges) => "max_edges",
		None => "none",
	}
}

fn truncation_value(value: Option<GraphTruncation>) -> Option<&'static str> {
	match value {
		Some(GraphTruncation::MaxDepth) => Some("max_depth"),
		Some(GraphTruncation::MaxNodes) => Some("max_nodes"),
		Some(GraphTruncation::MaxEdges) => Some("max_edges"),
		None => None,
	}
}

#[derive(serde::Serialize)]
struct GraphJson {
	path: String,
	root: String,
	root_ptr: String,
	truncated: Option<String>,
	nodes: Vec<GraphNodeJson>,
	edges: Vec<GraphEdgeJson>,
}

#[derive(serde::Serialize)]
struct GraphNodeJson {
	canonical: String,
	code: String,
	sdna_nr: u32,
	#[serde(rename = "type")]
	type_name: String,
	id: Option<String>,
}

#[derive(serde::Serialize)]
struct GraphEdgeJson {
	from: String,
	to: String,
	field: String,
}

#[cfg(test)]
mod tests;
