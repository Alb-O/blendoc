use std::collections::HashMap;
use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, IdIndex, RouteOptions, RouteResult, RouteTruncation, find_route_between_ptrs, scan_id_blocks};

use crate::cmd::util::{IdOrPtrSelector, RootSelector, json_escape, parse_id_or_ptr_selector, parse_root_selector, render_code, str_json};

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
	println!("{{");
	println!("  \"path\": \"{}\",", json_escape(&path.display().to_string()));
	println!("  \"from\": {{");
	println!("    \"selector\": \"{}\",", json_escape(from_label));
	println!("    \"canonical\": \"0x{:016x}\",", from.canonical);
	println!("    \"type\": \"{}\",", json_escape(&from.type_name));
	println!("    \"id\": {}", str_json(from.id_name.as_deref().map(json_escape).as_deref()));
	println!("  }},");
	println!("  \"to\": {{");
	println!("    \"selector\": \"{}\",", json_escape(to_label));
	println!("    \"canonical\": \"0x{:016x}\",", to.canonical);
	println!("    \"type\": \"{}\",", json_escape(&to.type_name));
	println!("    \"id\": {}", str_json(to.id_name.as_deref().map(json_escape).as_deref()));
	println!("  }},");
	println!("  \"visited_nodes\": {},", result.visited_nodes);
	println!("  \"visited_edges\": {},", result.visited_edges);
	println!("  \"truncated\": {},", str_json(result.truncated.map(truncation_label)));
	println!("  \"path_edges\": [");
	if let Some(path_edges) = &result.path {
		for (idx, edge) in path_edges.iter().enumerate() {
			let comma = if idx + 1 == path_edges.len() { "" } else { "," };
			println!(
				"    {{\"from\":\"0x{:016x}\",\"to\":\"0x{:016x}\",\"field\":\"{}\"}}{}",
				edge.from,
				edge.to,
				json_escape(&edge.field),
				comma,
			);
		}
	}
	println!("  ]");
	println!("}}");
}
