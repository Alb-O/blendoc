use std::collections::HashMap;
use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, GraphOptions, GraphResult, GraphTruncation, IdIndex, build_graph_from_ptr, scan_id_blocks};

/// Build and print a shallow pointer graph from one root selector.
pub fn run(
	path: PathBuf,
	code: Option<String>,
	ptr: Option<String>,
	id_name: Option<String>,
	depth: Option<u32>,
	refs_depth: Option<u32>,
	max_nodes: Option<usize>,
	max_edges: Option<usize>,
	id_only: bool,
	dot: bool,
	json: bool,
) -> blendoc::blend::Result<()> {
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

enum RootSelector {
	Code([u8; 4]),
	Ptr(u64),
	Id(String),
}

fn parse_root_selector(code: Option<String>, ptr: Option<String>, id_name: Option<String>) -> blendoc::blend::Result<RootSelector> {
	let supplied = usize::from(code.is_some()) + usize::from(ptr.is_some()) + usize::from(id_name.is_some());
	if supplied != 1 {
		return Err(BlendError::InvalidChaseRoot);
	}

	if let Some(code) = code {
		return Ok(RootSelector::Code(parse_block_code(&code)?));
	}
	if let Some(ptr) = ptr {
		return Ok(RootSelector::Ptr(parse_ptr(&ptr)?));
	}
	if let Some(id_name) = id_name {
		return Ok(RootSelector::Id(id_name));
	}

	Err(BlendError::InvalidChaseRoot)
}

fn parse_block_code(code: &str) -> blendoc::blend::Result<[u8; 4]> {
	if code.is_empty() || code.len() > 4 || !code.is_ascii() {
		return Err(BlendError::InvalidBlockCode { code: code.to_owned() });
	}

	let mut out = [0_u8; 4];
	out[..code.len()].copy_from_slice(code.as_bytes());
	Ok(out)
}

fn parse_ptr(value: &str) -> blendoc::blend::Result<u64> {
	let parsed = if let Some(stripped) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
		u64::from_str_radix(stripped, 16)
	} else {
		value.parse::<u64>()
	};

	parsed.map_err(|_| BlendError::InvalidPointerLiteral { value: value.to_owned() })
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
	println!("{{");
	println!("  \"path\": \"{}\",", json_escape(&path.display().to_string()));
	println!("  \"root\": \"{}\",", json_escape(root_label));
	println!("  \"root_ptr\": \"0x{root_ptr:016x}\",");
	println!("  \"truncated\": {},", truncation_json(graph.truncated));
	println!("  \"nodes\": [");
	for (idx, node) in graph.nodes.iter().enumerate() {
		let comma = if idx + 1 == graph.nodes.len() { "" } else { "," };
		println!(
			"    {{\"canonical\":\"0x{:016x}\",\"code\":\"{}\",\"sdna_nr\":{},\"type\":\"{}\",\"id\":{}}}{}",
			node.canonical,
			json_escape(&render_code(node.code)),
			node.sdna_nr,
			json_escape(&node.type_name),
			str_json(node.id_name.as_deref().map(json_escape).as_deref()),
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

fn truncation_json(value: Option<GraphTruncation>) -> &'static str {
	match value {
		Some(GraphTruncation::MaxDepth) => "\"max_depth\"",
		Some(GraphTruncation::MaxNodes) => "\"max_nodes\"",
		Some(GraphTruncation::MaxEdges) => "\"max_edges\"",
		None => "null",
	}
}

fn render_code(code: [u8; 4]) -> String {
	let mut out = String::new();
	for byte in code {
		if byte == 0 {
			continue;
		}
		if byte.is_ascii_graphic() || byte == b' ' {
			out.push(char::from(byte));
		} else {
			out.push('.');
		}
	}
	if out.is_empty() { "....".to_owned() } else { out }
}

fn str_json(value: Option<&str>) -> String {
	match value {
		Some(item) => format!("\"{item}\""),
		None => "null".to_owned(),
	}
}

fn dot_escape(input: &str) -> String {
	input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn json_escape(input: &str) -> String {
	let mut out = String::with_capacity(input.len());
	for ch in input.chars() {
		match ch {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'\n' => out.push_str("\\n"),
			'\r' => out.push_str("\\r"),
			'\t' => out.push_str("\\t"),
			c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
			c => out.push(c),
		}
	}
	out
}
