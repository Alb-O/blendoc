use std::path::PathBuf;

use blendoc::blend::{
	BlendError, BlendFile, ChasePolicy, DecodeOptions, FieldPath, IdIndex, Value, chase_from_ptr, decode_ptr_instance, scan_id_blocks, scan_id_link_provenance,
};

use crate::cmd::print::{PrintCtx, PrintOptions, PtrAnnotCtx, print_value};
use crate::cmd::util::{RootSelector, emit_json, parse_root_selector, ptr_hex, render_code};

#[derive(clap::Args)]
pub struct Args {
	pub file: PathBuf,
	#[arg(long = "id")]
	pub id_name: Option<String>,
	#[arg(long)]
	pub ptr: Option<String>,
	#[arg(long)]
	pub code: Option<String>,
	#[arg(long = "path")]
	pub path_expr: Option<String>,
	#[arg(long)]
	pub trace: bool,
	#[arg(long)]
	pub json: bool,
	#[arg(long = "max-depth")]
	pub max_depth: Option<u32>,
	#[arg(long = "max-array")]
	pub max_array: Option<usize>,
	#[arg(long = "include-padding")]
	pub include_padding: bool,
	#[arg(long = "strict-layout")]
	pub strict_layout: bool,
	#[arg(long = "annotate-ptrs", default_value_t = true)]
	pub annotate_ptrs: bool,
	#[arg(long = "raw-ptrs")]
	pub raw_ptrs: bool,
	#[arg(long = "expand-depth", default_value_t = 0)]
	pub expand_depth: u32,
	#[arg(long = "expand-max-nodes", default_value_t = 64)]
	pub expand_max_nodes: usize,
}

/// Decode and print a struct/value from ID, pointer, or block code roots.
pub fn run(args: Args) -> blendoc::blend::Result<()> {
	let Args {
		file: path,
		id_name,
		ptr,
		code,
		path_expr,
		trace,
		json,
		max_depth,
		max_array,
		include_padding,
		strict_layout,
		annotate_ptrs,
		raw_ptrs,
		expand_depth,
		expand_max_nodes,
	} = args;

	let selector = parse_root_selector(code, ptr, id_name)?;

	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;
	let index = blend.pointer_index()?;
	let ids = IdIndex::build(scan_id_blocks(&blend, &dna)?);

	let (root_ptr, root_label) = match selector {
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

	let mut decode = DecodeOptions::default();
	if let Some(max_depth) = max_depth {
		decode.max_depth = max_depth;
	}
	if let Some(max_array) = max_array {
		decode.max_array_elems = max_array;
	}
	decode.include_padding = include_padding;
	decode.strict_layout = strict_layout;

	let mut print = PrintOptions::default();
	if let Some(max_depth) = max_depth {
		print.max_print_depth = max_depth;
	}
	if let Some(max_array) = max_array {
		print.max_array_items = max_array;
	}

	let effective_expand_depth = if raw_ptrs { 0 } else { expand_depth };
	let print_ctx = PrintCtx::new(
		Some(PtrAnnotCtx {
			dna: &dna,
			index: &index,
			ids: &ids,
		}),
		annotate_ptrs && !raw_ptrs,
		Some(&decode),
		expand_max_nodes,
	);

	let root_link = if json {
		let links = scan_id_link_provenance(&blend, &dna)?;
		let root_canonical = index.canonical_ptr(&dna, root_ptr).unwrap_or(root_ptr);
		links
			.iter()
			.find(|item| item.id_ptr == root_canonical)
			.map(|item| (item.linked, item.confidence.as_str().to_owned()))
	} else {
		None
	};

	if let Some(path_expr) = path_expr {
		let field_path = FieldPath::parse(&path_expr)?;
		let result = chase_from_ptr(&dna, &index, root_ptr, &field_path, &decode, &ChasePolicy::default())?;
		let json_root = JsonRootMeta {
			path: &path,
			root_label: &root_label,
			root_ptr,
			root_link: root_link.as_ref(),
		};

		if json {
			print_json_path(&json_root, &path_expr, &result.value, result.stop.as_ref(), trace.then_some(&result.hops));
			return Ok(());
		}

		println!("path: {}", path.display());
		println!("root: {root_label}");
		println!("root_ptr: 0x{root_ptr:016x}");
		println!("path_expr: {path_expr}");
		println!("value:");
		print_value(&result.value, 2, 0, print, Some(&print_ctx), effective_expand_depth);

		if trace {
			println!("hops: {}", result.hops.len());
			for (idx, hop) in result.hops.iter().enumerate() {
				println!(
					"  {idx}: ptr=0x{:016x} code={} sdna={} element={} offset={}",
					hop.ptr,
					render_code(hop.resolved_block_code),
					hop.sdna_nr,
					hop.element_index,
					hop.element_offset
				);
			}
		}

		if let Some(stop) = result.stop {
			println!("stop_step: {}", stop.step_index);
			println!("stop_reason: {:?}", stop.reason);
		}

		return Ok(());
	}

	let (canonical, struct_value) = decode_ptr_instance(&dna, &index, root_ptr, &decode)?;
	let node_id = ids.get_by_ptr(canonical).map(|item| item.id_name.as_ref());

	if json {
		let value = Value::Struct(struct_value);
		let canonical_link = root_link
			.as_ref()
			.filter(|_| canonical == index.canonical_ptr(&dna, root_ptr).unwrap_or(root_ptr));
		let json_root = JsonRootMeta {
			path: &path,
			root_label: &root_label,
			root_ptr,
			root_link: canonical_link,
		};
		print_json_struct(&json_root, canonical, node_id, &value);
		return Ok(());
	}

	println!("path: {}", path.display());
	println!("root: {root_label}");
	println!("root_ptr: 0x{root_ptr:016x}");
	println!("canonical: 0x{canonical:016x}");
	println!("id_name: {}", node_id.unwrap_or("-"));
	println!("value:");
	print_value(&Value::Struct(struct_value), 2, 0, print, Some(&print_ctx), effective_expand_depth);

	Ok(())
}

fn print_json_struct(root: &JsonRootMeta<'_>, canonical: u64, id_name: Option<&str>, value: &Value) {
	let payload = ShowStructJson {
		path: root.path.display().to_string(),
		root: root.root_label.to_owned(),
		root_ptr: ptr_hex(root.root_ptr),
		canonical: ptr_hex(canonical),
		id_name: id_name.map(str::to_owned),
		root_linked: root.root_link.map(|item| item.0),
		root_link_confidence: root.root_link.map(|item| item.1.clone()),
		value: value_to_json_value(value),
	};

	emit_json(&payload);
}

fn print_json_path(
	root: &JsonRootMeta<'_>,
	path_expr: &str,
	value: &Value,
	stop: Option<&blendoc::blend::ChaseStop>,
	hops: Option<&Vec<blendoc::blend::ChaseMeta>>,
) {
	let payload = ShowPathJson {
		path: root.path.display().to_string(),
		root: root.root_label.to_owned(),
		root_ptr: ptr_hex(root.root_ptr),
		path_expr: path_expr.to_owned(),
		root_linked: root.root_link.map(|item| item.0),
		root_link_confidence: root.root_link.map(|item| item.1.clone()),
		value: value_to_json_value(value),
		stop: stop.map(|stop| ShowStopJson {
			step: stop.step_index,
			reason: format!("{:?}", stop.reason),
		}),
		hops: hops.map(|items| {
			items
				.iter()
				.map(|hop| ShowHopJson {
					ptr: ptr_hex(hop.ptr),
					code: render_code(hop.resolved_block_code),
					sdna: hop.sdna_nr,
					element: hop.element_index,
					offset: hop.element_offset,
				})
				.collect()
		}),
	};

	emit_json(&payload);
}

struct JsonRootMeta<'a> {
	path: &'a std::path::Path,
	root_label: &'a str,
	root_ptr: u64,
	root_link: Option<&'a (bool, String)>,
}

fn value_to_json_value(value: &Value) -> serde_json::Value {
	use serde_json::{Map, Value as JsonValue};

	match value {
		Value::Null => JsonValue::Null,
		Value::Bool(v) => serde_json::json!(v),
		Value::I64(v) => serde_json::json!(v),
		Value::U64(v) => serde_json::json!(v),
		Value::F32(v) => serde_json::json!(v),
		Value::F64(v) => serde_json::json!(v),
		Value::Bytes(v) => {
			let bytes: Vec<JsonValue> = v.iter().map(|item| serde_json::json!(item)).collect();
			JsonValue::Array(bytes)
		}
		Value::String(v) => serde_json::json!(v),
		Value::Ptr(v) => serde_json::json!(ptr_hex(*v)),
		Value::Array(items) => {
			let values: Vec<JsonValue> = items.iter().map(value_to_json_value).collect();
			JsonValue::Array(values)
		}
		Value::Struct(item) => {
			let fields: Map<String, JsonValue> = item
				.fields
				.iter()
				.map(|field| (field.name.to_string(), value_to_json_value(&field.value)))
				.collect();

			let mut out = Map::new();
			out.insert("type".to_owned(), serde_json::json!(item.type_name.as_ref()));
			out.insert("fields".to_owned(), JsonValue::Object(fields));
			JsonValue::Object(out)
		}
	}
}

#[derive(serde::Serialize)]
struct ShowStructJson {
	path: String,
	root: String,
	root_ptr: String,
	canonical: String,
	id_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	root_linked: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	root_link_confidence: Option<String>,
	value: serde_json::Value,
}

#[derive(serde::Serialize)]
struct ShowStopJson {
	step: usize,
	reason: String,
}

#[derive(serde::Serialize)]
struct ShowHopJson {
	ptr: String,
	code: String,
	sdna: u32,
	element: usize,
	offset: usize,
}

#[derive(serde::Serialize)]
struct ShowPathJson {
	path: String,
	root: String,
	root_ptr: String,
	path_expr: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	root_linked: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	root_link_confidence: Option<String>,
	value: serde_json::Value,
	stop: Option<ShowStopJson>,
	hops: Option<Vec<ShowHopJson>>,
}

#[cfg(test)]
mod tests;
