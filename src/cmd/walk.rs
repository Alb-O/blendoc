use std::sync::Arc;

use blendoc::blend::{
	BlendError, BlendFile, ChasePolicy, DecodeOptions, FieldPath, IdIndex, StopMode, Value, WalkOptions, WalkStopReason, chase_from_ptr, scan_id_blocks,
	walk_ptr_chain,
};

use crate::cmd::util::{RootSelector, json_escape, parse_root_selector, render_code, str_json};

#[derive(clap::Args)]
pub struct Args {
	pub file: std::path::PathBuf,
	#[arg(long = "id")]
	pub id_name: Option<String>,
	#[arg(long)]
	pub ptr: Option<String>,
	#[arg(long)]
	pub code: Option<String>,
	#[arg(long = "path")]
	pub path_expr: Option<String>,
	#[arg(long = "next", default_value = "next")]
	pub next_field: String,
	#[arg(long = "refs-depth")]
	pub refs_depth: Option<u32>,
	#[arg(long = "limit")]
	pub limit: Option<usize>,
	#[arg(long)]
	pub json: bool,
}

/// Walk linked pointer chains from an ID/pointer/code root.
pub fn run(args: Args) -> blendoc::blend::Result<()> {
	let Args {
		file: path,
		id_name,
		ptr,
		code,
		path_expr,
		next_field,
		refs_depth,
		limit,
		json,
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

	let start_ptr = if let Some(path_expr) = path_expr {
		let mut decode = DecodeOptions::for_scene_inspect();
		decode.include_padding = true;

		let path = FieldPath::parse(&path_expr)?;
		let result = chase_from_ptr(&dna, &index, root_ptr, &path, &decode, &ChasePolicy::default())?;
		match result.value {
			Value::Ptr(ptr) => ptr,
			Value::Struct(_) if !result.hops.is_empty() => canonical_from_hop(result.hops.last().expect("hops checked"))?,
			other => {
				return Err(BlendError::WalkInvalidStart {
					got: value_kind(&other).to_owned(),
				});
			}
		}
	} else {
		root_ptr
	};

	let mut options = WalkOptions {
		next_field: Arc::<str>::from(next_field.as_str()),
		max_steps: 256,
		ref_scan: Default::default(),
		on_null: StopMode::Stop,
		on_unresolved: StopMode::Stop,
		on_cycle: StopMode::Stop,
	};
	if let Some(refs_depth) = refs_depth {
		options.ref_scan.max_depth = refs_depth;
	}
	if let Some(limit) = limit {
		options.max_steps = limit;
	}

	let result = walk_ptr_chain(&dna, &index, &ids, start_ptr, &options)?;

	if json {
		print_json(&path, &root_label, start_ptr, &next_field, &result);
		return Ok(());
	}

	println!("path: {}", path.display());
	println!("root: {root_label}");
	println!("start_ptr: 0x{start_ptr:016x}");
	println!("next_field: {next_field}");
	println!("items: {}", result.items.len());
	println!("idx\tcanonical\tcode\tsdna\ttype\tid");
	for item in &result.items {
		println!(
			"{}\t0x{:016x}\t{}\t{}\t{}\t{}",
			item.index,
			item.canonical,
			render_code(item.code),
			item.sdna_nr,
			item.type_name,
			item.id_name.as_deref().unwrap_or("-")
		);
	}

	if let Some(stop) = &result.stop {
		println!("stop_step: {}", stop.step);
		println!("stop_reason: {}", stop_reason_label(&stop.reason));
	} else {
		println!("stop_reason: none");
	}

	Ok(())
}

fn canonical_from_hop(hop: &blendoc::blend::ChaseMeta) -> blendoc::blend::Result<u64> {
	let offset = hop
		.element_index
		.checked_mul(hop.struct_size)
		.ok_or(BlendError::ChasePtrOutOfBounds { ptr: hop.ptr })?;
	let offset = u64::try_from(offset).map_err(|_| BlendError::ChasePtrOutOfBounds { ptr: hop.ptr })?;
	hop.block_old.checked_add(offset).ok_or(BlendError::ChasePtrOutOfBounds { ptr: hop.ptr })
}

fn value_kind(value: &Value) -> &'static str {
	match value {
		Value::Null => "Null",
		Value::Bool(_) => "Bool",
		Value::I64(_) => "I64",
		Value::U64(_) => "U64",
		Value::F32(_) => "F32",
		Value::F64(_) => "F64",
		Value::Bytes(_) => "Bytes",
		Value::String(_) => "String",
		Value::Ptr(_) => "Ptr",
		Value::Array(_) => "Array",
		Value::Struct(_) => "Struct",
	}
}

fn stop_reason_label(reason: &WalkStopReason) -> String {
	match reason {
		WalkStopReason::NullNext => "NullNext".to_owned(),
		WalkStopReason::UnresolvedNext(ptr) => format!("UnresolvedNext(0x{ptr:016x})"),
		WalkStopReason::Cycle(ptr) => format!("Cycle(0x{ptr:016x})"),
		WalkStopReason::MissingNextField { field } => format!("MissingNextField({field})"),
	}
}

fn print_json(path: &std::path::Path, root_label: &str, start_ptr: u64, next_field: &str, result: &blendoc::blend::WalkResult) {
	println!("{{");
	println!("  \"path\": \"{}\",", json_escape(&path.display().to_string()));
	println!("  \"root\": \"{}\",", json_escape(root_label));
	println!("  \"start_ptr\": \"0x{start_ptr:016x}\",",);
	println!("  \"next_field\": \"{}\",", json_escape(next_field));
	println!("  \"items\": [");
	for (idx, item) in result.items.iter().enumerate() {
		let comma = if idx + 1 == result.items.len() { "" } else { "," };
		println!(
			"    {{\"index\":{},\"canonical\":\"0x{:016x}\",\"code\":\"{}\",\"sdna\":{},\"type\":\"{}\",\"id\":{}}}{}",
			item.index,
			item.canonical,
			json_escape(&render_code(item.code)),
			item.sdna_nr,
			json_escape(&item.type_name),
			str_json(item.id_name.as_deref().map(json_escape).as_deref()),
			comma,
		);
	}
	println!("  ],");
	if let Some(stop) = &result.stop {
		println!(
			"  \"stop\": {{\"step\":{},\"reason\":\"{}\"}}",
			stop.step,
			json_escape(&stop_reason_label(&stop.reason))
		);
	} else {
		println!("  \"stop\": null");
	}
	println!("}}");
}
