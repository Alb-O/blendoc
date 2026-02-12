use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, ChasePolicy, DecodeOptions, FieldPath, IdIndex, Value, chase_from_ptr, decode_ptr_instance, scan_id_blocks};

use crate::cmd::print::{PrintCtx, PrintOptions, PtrAnnotCtx, print_value};
use crate::cmd::util::{RootSelector, json_escape, parse_root_selector, render_code, str_json};

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

	if let Some(path_expr) = path_expr {
		let field_path = FieldPath::parse(&path_expr)?;
		let result = chase_from_ptr(&dna, &index, root_ptr, &field_path, &decode, &ChasePolicy::default())?;

		if json {
			print_json_path(
				&path,
				&root_label,
				root_ptr,
				&path_expr,
				&result.value,
				result.stop.as_ref(),
				trace.then_some(&result.hops),
			);
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
		print_json_struct(&path, &root_label, root_ptr, canonical, node_id, &value);
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

fn print_json_struct(path: &std::path::Path, root_label: &str, root_ptr: u64, canonical: u64, id_name: Option<&str>, value: &Value) {
	println!("{{");
	println!("  \"path\": \"{}\",", json_escape(&path.display().to_string()));
	println!("  \"root\": \"{}\",", json_escape(root_label));
	println!("  \"root_ptr\": \"0x{root_ptr:016x}\",");
	println!("  \"canonical\": \"0x{canonical:016x}\",");
	println!("  \"id_name\": {},", str_json(id_name.map(json_escape).as_deref()));
	println!("  \"value\": {}", value_to_json(value));
	println!("}}");
}

fn print_json_path(
	path: &std::path::Path,
	root_label: &str,
	root_ptr: u64,
	path_expr: &str,
	value: &Value,
	stop: Option<&blendoc::blend::ChaseStop>,
	hops: Option<&Vec<blendoc::blend::ChaseMeta>>,
) {
	println!("{{");
	println!("  \"path\": \"{}\",", json_escape(&path.display().to_string()));
	println!("  \"root\": \"{}\",", json_escape(root_label));
	println!("  \"root_ptr\": \"0x{root_ptr:016x}\",",);
	println!("  \"path_expr\": \"{}\",", json_escape(path_expr));
	println!("  \"value\": {},", value_to_json(value));
	if let Some(stop) = stop {
		println!(
			"  \"stop\": {{\"step\":{},\"reason\":\"{}\"}},",
			stop.step_index,
			json_escape(&format!("{:?}", stop.reason))
		);
	} else {
		println!("  \"stop\": null,");
	}
	if let Some(hops) = hops {
		println!("  \"hops\": [");
		for (idx, hop) in hops.iter().enumerate() {
			let comma = if idx + 1 == hops.len() { "" } else { "," };
			println!(
				"    {{\"ptr\":\"0x{:016x}\",\"code\":\"{}\",\"sdna\":{},\"element\":{},\"offset\":{}}}{}",
				hop.ptr,
				json_escape(&render_code(hop.resolved_block_code)),
				hop.sdna_nr,
				hop.element_index,
				hop.element_offset,
				comma,
			);
		}
		println!("  ]");
	} else {
		println!("  \"hops\": null");
	}
	println!("}}");
}

fn value_to_json(value: &Value) -> String {
	match value {
		Value::Null => "null".to_owned(),
		Value::Bool(v) => v.to_string(),
		Value::I64(v) => v.to_string(),
		Value::U64(v) => v.to_string(),
		Value::F32(v) => v.to_string(),
		Value::F64(v) => v.to_string(),
		Value::Bytes(v) => {
			let bytes: Vec<String> = v.iter().map(|item| item.to_string()).collect();
			format!("[{}]", bytes.join(","))
		}
		Value::String(v) => format!("\"{}\"", json_escape(v)),
		Value::Ptr(v) => format!("\"0x{v:016x}\""),
		Value::Array(items) => {
			let values: Vec<String> = items.iter().map(value_to_json).collect();
			format!("[{}]", values.join(","))
		}
		Value::Struct(item) => {
			let mut fields = Vec::new();
			fields.push(format!("\"type\":\"{}\"", json_escape(&item.type_name)));
			let entries: Vec<String> = item
				.fields
				.iter()
				.map(|field| format!("\"{}\":{}", json_escape(&field.name), value_to_json(&field.value)))
				.collect();
			fields.push(format!("\"fields\":{{{}}}", entries.join(",")));
			format!("{{{}}}", fields.join(","))
		}
	}
}
