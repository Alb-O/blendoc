use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use blendoc::blend::{BlendFile, IdGraphOptions, IdGraphResult, IdGraphTruncation, IdIndex, build_id_graph, scan_id_blocks};

use crate::cmd::util::{dot_escape, json_escape, render_code};

/// Build and print whole-file ID-to-ID graph.
pub fn run(
	path: PathBuf,
	refs_depth: Option<u32>,
	max_edges: Option<usize>,
	dot: bool,
	json: bool,
	prefix: Option<String>,
	type_name: Option<String>,
) -> blendoc::blend::Result<()> {
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
	println!("{{");
	println!("  \"path\": \"{}\",", json_escape(&path.display().to_string()));
	println!("  \"truncated\": {},", truncation_json(graph.truncated));
	println!("  \"nodes\": [");
	for (idx, node) in graph.nodes.iter().enumerate() {
		let comma = if idx + 1 == graph.nodes.len() { "" } else { "," };
		println!(
			"    {{\"canonical\":\"0x{:016x}\",\"code\":\"{}\",\"sdna_nr\":{},\"type\":\"{}\",\"id\":\"{}\"}}{}",
			node.canonical,
			json_escape(&render_code(node.code)),
			node.sdna_nr,
			json_escape(&node.type_name),
			json_escape(&node.id_name),
			comma,
		);
	}
	println!("  ],");
	println!("  \"edges\": [");
	for (idx, edge) in graph.edges.iter().enumerate() {
		let comma = if idx + 1 == graph.edges.len() { "" } else { "," };
		println!(
			"    {{\"from\":\"0x{:016x}\",\"to\":\"0x{:016x}\",\"field\":\"{}\"}}{}",
			edge.from,
			edge.to,
			json_escape(&edge.field),
			comma,
		);
	}
	println!("  ]");
	println!("}}");
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

fn truncation_json(value: Option<IdGraphTruncation>) -> &'static str {
	match value {
		Some(IdGraphTruncation::MaxEdges) => "\"max_edges\"",
		None => "null",
	}
}
