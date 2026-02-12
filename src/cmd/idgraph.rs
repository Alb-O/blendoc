use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use blendoc::blend::{BlendFile, IdGraphOptions, IdGraphResult, IdGraphTruncation, IdIndex, build_id_graph, scan_id_blocks};

use crate::cmd::util::{dot_escape, emit_json, ptr_hex, render_code};

#[derive(clap::Args)]
pub struct Args {
	pub file: PathBuf,
	#[arg(long = "refs-depth")]
	pub refs_depth: Option<u32>,
	#[arg(long = "max-edges")]
	pub max_edges: Option<usize>,
	#[arg(long)]
	pub dot: bool,
	#[arg(long)]
	pub json: bool,
	#[arg(long)]
	pub prefix: Option<String>,
	#[arg(long = "type")]
	pub type_name: Option<String>,
}

/// Build and print whole-file ID-to-ID graph.
pub fn run(args: Args) -> blendoc::blend::Result<()> {
	let Args {
		file: path,
		refs_depth,
		max_edges,
		dot,
		json,
		prefix,
		type_name,
	} = args;

	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;
	let index = blend.pointer_index()?;
	let ids = IdIndex::build(scan_id_blocks(&blend, &dna)?);

	let mut options = IdGraphOptions::default();
	if let Some(refs_depth) = refs_depth {
		options.ref_scan.max_depth = refs_depth;
	}
	if let Some(max_edges) = max_edges {
		options.max_edges = max_edges;
	}

	let raw = build_id_graph(&dna, &index, &ids, &options)?;
	let graph = apply_filters(raw, prefix.as_deref(), type_name.as_deref());

	if json {
		print_json(&path, &graph);
		return Ok(());
	}
	if dot {
		print_dot(&graph);
		return Ok(());
	}

	print_text(&path, &graph);
	Ok(())
}

fn apply_filters(mut graph: IdGraphResult, prefix: Option<&str>, type_name: Option<&str>) -> IdGraphResult {
	if prefix.is_none() && type_name.is_none() {
		return graph;
	}

	let mut keep = HashSet::new();
	for node in &graph.nodes {
		let matches_prefix = prefix.is_none_or(|value| node.id_name.starts_with(value));
		let matches_type = type_name.is_none_or(|value| node.type_name.as_ref() == value);
		if matches_prefix && matches_type {
			keep.insert(node.canonical);
		}
	}

	graph.nodes.retain(|node| keep.contains(&node.canonical));
	graph.edges.retain(|edge| keep.contains(&edge.from) && keep.contains(&edge.to));
	graph
}

fn print_text(path: &std::path::Path, graph: &IdGraphResult) {
	println!("path: {}", path.display());
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

fn print_dot(graph: &IdGraphResult) {
	println!("digraph blendoc_idgraph {{");
	for node in &graph.nodes {
		let label = format!("{}\\n{}", node.id_name, node.type_name);
		println!("  \"0x{:016x}\" [label=\"{}\"]", node.canonical, dot_escape(&label));
	}
	for edge in &graph.edges {
		println!("  \"0x{:016x}\" -> \"0x{:016x}\" [label=\"{}\"]", edge.from, edge.to, dot_escape(&edge.field));
	}
	println!("}}");
}

fn print_json(path: &std::path::Path, graph: &IdGraphResult) {
	let payload = IdGraphJson {
		path: path.display().to_string(),
		truncated: truncation_value(graph.truncated).map(str::to_owned),
		nodes: graph
			.nodes
			.iter()
			.map(|node| IdGraphNodeJson {
				canonical: ptr_hex(node.canonical),
				code: render_code(node.code),
				sdna_nr: node.sdna_nr,
				type_name: node.type_name.to_string(),
				id: node.id_name.to_string(),
			})
			.collect(),
		edges: graph
			.edges
			.iter()
			.map(|edge| IdGraphEdgeJson {
				from: ptr_hex(edge.from),
				to: ptr_hex(edge.to),
				field: edge.field.to_string(),
			})
			.collect(),
	};

	emit_json(&payload);
}

fn node_label(node: Option<&blendoc::blend::IdGraphNode>) -> String {
	let Some(node) = node else {
		return "<unknown>".to_owned();
	};
	format!("{}({})", node.id_name, node.type_name)
}

fn truncation_label(value: Option<IdGraphTruncation>) -> &'static str {
	match value {
		Some(IdGraphTruncation::MaxEdges) => "max_edges",
		None => "none",
	}
}

fn truncation_value(value: Option<IdGraphTruncation>) -> Option<&'static str> {
	match value {
		Some(IdGraphTruncation::MaxEdges) => Some("max_edges"),
		None => None,
	}
}

#[derive(serde::Serialize)]
struct IdGraphJson {
	path: String,
	truncated: Option<String>,
	nodes: Vec<IdGraphNodeJson>,
	edges: Vec<IdGraphEdgeJson>,
}

#[derive(serde::Serialize)]
struct IdGraphNodeJson {
	canonical: String,
	code: String,
	sdna_nr: u32,
	#[serde(rename = "type")]
	type_name: String,
	id: String,
}

#[derive(serde::Serialize)]
struct IdGraphEdgeJson {
	from: String,
	to: String,
	field: String,
}
