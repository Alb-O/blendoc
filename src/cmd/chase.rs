use std::path::PathBuf;

use blendoc::blend::{
	BlendError, BlendFile, ChaseMeta, ChasePolicy, ChaseResult, ChaseStopReason, DecodeOptions, FieldPath, IdIndex, Value, chase_from_block_code,
	chase_from_ptr, scan_id_blocks,
};

/// Execute path chase from a selected root and print hop trace.
pub fn run(path: PathBuf, code: Option<String>, ptr: Option<String>, id_name: Option<String>, path_expr: String, json: bool) -> blendoc::blend::Result<()> {
	let root = parse_root_selector(code, ptr, id_name)?;

	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;
	let index = blend.pointer_index()?;
	let ids = IdIndex::build(scan_id_blocks(&blend, &dna)?);

	let mut decode = DecodeOptions::for_scene_inspect();
	decode.include_padding = true;
	decode.strict_layout = true;

	let parsed_path = FieldPath::parse(&path_expr)?;
	let policy = ChasePolicy::default();

	let (result, root_info) = match root {
		RootSelector::Code(block_code) => {
			let block = blend
				.find_first_block_by_code(block_code)?
				.ok_or(BlendError::BlockNotFound { code: block_code })?;
			let type_name = dna
				.struct_by_sdna(block.head.sdna_nr)
				.map(|item| dna.type_name(item.type_idx))
				.unwrap_or("<unknown>")
				.to_owned();
			let root_ptr = block.head.old;
			let result = chase_from_block_code(&blend, &dna, &index, block_code, &parsed_path, &decode, &policy)?;
			let root_info = RootInfo {
				selector: format!("code:{}", render_code(block_code)),
				ptr: Some(root_ptr),
				type_name: Some(type_name),
				id_name: ids.get_by_ptr(root_ptr).map(|item| item.id_name.to_string()),
			};
			(result, root_info)
		}
		RootSelector::Ptr(root_ptr) => {
			let typed_root = index.resolve_typed(&dna, root_ptr);
			let type_name = typed_root.and_then(|typed| {
				dna.struct_by_sdna(typed.base.entry.block.head.sdna_nr)
					.map(|item| dna.type_name(item.type_idx).to_owned())
			});
			let canonical_root = typed_root.and_then(|typed| {
				typed.element_index.and_then(|element_index| {
					let offset = element_index.checked_mul(typed.struct_size)?;
					let offset = u64::try_from(offset).ok()?;
					typed.base.entry.start_old.checked_add(offset)
				})
			});
			let result = chase_from_ptr(&dna, &index, root_ptr, &parsed_path, &decode, &policy)?;
			let root_info = RootInfo {
				selector: format!("ptr:0x{root_ptr:016x}"),
				ptr: Some(root_ptr),
				type_name,
				id_name: canonical_root.and_then(|ptr| ids.get_by_ptr(ptr)).map(|item| item.id_name.to_string()),
			};
			(result, root_info)
		}
		RootSelector::Id(name) => {
			let row = ids.get_by_name(&name).ok_or(BlendError::IdRecordNotFound { name: name.clone() })?;
			let root_ptr = row.old_ptr;
			let result = chase_from_ptr(&dna, &index, root_ptr, &parsed_path, &decode, &policy)?;
			let root_info = RootInfo {
				selector: format!("id:{}", row.id_name),
				ptr: Some(root_ptr),
				type_name: Some(row.type_name.to_string()),
				id_name: Some(row.id_name.to_string()),
			};
			(result, root_info)
		}
	};

	let hops = build_hop_trace(&result, &dna, &ids);

	if json {
		print_json(&path, &root_info, &path_expr, &hops, &result);
		return Ok(());
	}

	println!("path: {}", path.display());
	println!("root_selector: {}", root_info.selector);
	if let Some(ptr) = root_info.ptr {
		println!("root_ptr: 0x{ptr:016x}");
	}
	if let Some(type_name) = &root_info.type_name {
		println!("root_type: {type_name}");
	}
	if let Some(id_name) = &root_info.id_name {
		println!("root_id_name: {id_name}");
	}
	println!("path_expr: {path_expr}");
	println!("hops: {}", hops.len());
	for hop in &hops {
		println!(
			"  {}: ptr=0x{:016x} canonical={} code={} sdna={} type={} id={}",
			hop.index,
			hop.ptr,
			format_ptr_opt(hop.canonical),
			render_code(hop.code),
			hop.sdna_nr,
			hop.type_name,
			hop.id_name.as_deref().unwrap_or("-")
		);
	}

	println!("result_kind: {}", value_kind(&result.value));
	if let Value::Struct(item) = &result.value {
		println!("result_type: {}", item.type_name);
	}

	if let Some(stop) = &result.stop {
		println!("stop_step: {}", stop.step_index);
		println!("stop_reason: {}", format_stop_reason(&stop.reason));
	} else {
		println!("stop_reason: none");
	}

	Ok(())
}

#[derive(Debug, Clone)]
struct RootInfo {
	selector: String,
	ptr: Option<u64>,
	type_name: Option<String>,
	id_name: Option<String>,
}

#[derive(Debug, Clone)]
struct HopTrace {
	index: usize,
	ptr: u64,
	canonical: Option<u64>,
	code: [u8; 4],
	sdna_nr: u32,
	type_name: String,
	id_name: Option<String>,
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

fn build_hop_trace(result: &ChaseResult, dna: &blendoc::blend::Dna, ids: &IdIndex) -> Vec<HopTrace> {
	result
		.hops
		.iter()
		.enumerate()
		.map(|(index, hop)| {
			let canonical = canonical_ptr(hop);
			let type_name = dna
				.struct_by_sdna(hop.sdna_nr)
				.map(|item| dna.type_name(item.type_idx))
				.unwrap_or("<unknown>")
				.to_owned();
			let id_name = canonical.and_then(|ptr| ids.get_by_ptr(ptr)).map(|item| item.id_name.to_string());

			HopTrace {
				index,
				ptr: hop.ptr,
				canonical,
				code: hop.resolved_block_code,
				sdna_nr: hop.sdna_nr,
				type_name,
				id_name,
			}
		})
		.collect()
}

fn canonical_ptr(meta: &ChaseMeta) -> Option<u64> {
	let offset = meta.element_index.checked_mul(meta.struct_size)?;
	let offset = u64::try_from(offset).ok()?;
	meta.block_old.checked_add(offset)
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

fn format_stop_reason(reason: &ChaseStopReason) -> String {
	match reason {
		ChaseStopReason::NullPtr => "NullPtr".to_owned(),
		ChaseStopReason::UnresolvedPtr(ptr) => format!("UnresolvedPtr(0x{ptr:016x})"),
		ChaseStopReason::Cycle(ptr) => format!("Cycle(0x{ptr:016x})"),
		ChaseStopReason::MissingField { struct_name, field } => format!("MissingField({struct_name}.{field})"),
		ChaseStopReason::ExpectedStruct { got } => format!("ExpectedStruct(got={got})"),
		ChaseStopReason::ExpectedArray { got } => format!("ExpectedArray(got={got})"),
		ChaseStopReason::IndexOob { index, len } => format!("IndexOob(index={index},len={len})"),
	}
}

fn format_ptr_opt(ptr: Option<u64>) -> String {
	match ptr {
		Some(value) => format!("0x{value:016x}"),
		None => "-".to_owned(),
	}
}

fn print_json(path: &std::path::Path, root: &RootInfo, path_expr: &str, hops: &[HopTrace], result: &ChaseResult) {
	println!("{{");
	println!("  \"path\": \"{}\",", json_escape(&path.display().to_string()));
	println!("  \"root\": {{");
	println!("    \"selector\": \"{}\",", json_escape(&root.selector));
	println!("    \"ptr\": {},", ptr_json(root.ptr));
	println!("    \"type\": {},", str_json(root.type_name.as_deref().map(json_escape).as_deref()));
	println!("    \"id_name\": {}", str_json(root.id_name.as_deref().map(json_escape).as_deref()));
	println!("  }},");
	println!("  \"path_expr\": \"{}\",", json_escape(path_expr));
	println!("  \"hops\": [");
	for (idx, hop) in hops.iter().enumerate() {
		let comma = if idx + 1 == hops.len() { "" } else { "," };
		println!(
			"    {{\"index\":{},\"ptr\":\"0x{:016x}\",\"canonical\":{},\"code\":\"{}\",\"sdna_nr\":{},\"type\":\"{}\",\"id_name\":{}}}{}",
			hop.index,
			hop.ptr,
			ptr_json(hop.canonical),
			json_escape(&render_code(hop.code)),
			hop.sdna_nr,
			json_escape(&hop.type_name),
			str_json(hop.id_name.as_deref().map(json_escape).as_deref()),
			comma,
		);
	}
	println!("  ],");
	println!("  \"result\": {{");
	println!("    \"kind\": \"{}\",", value_kind(&result.value));
	if let Value::Struct(item) = &result.value {
		println!("    \"type\": \"{}\"", json_escape(&item.type_name));
	} else {
		println!("    \"type\": null");
	}
	println!("  }},");
	if let Some(stop) = &result.stop {
		println!(
			"  \"stop\": {{\"step\":{},\"reason\":\"{}\"}}",
			stop.step_index,
			json_escape(&format_stop_reason(&stop.reason))
		);
	} else {
		println!("  \"stop\": null");
	}
	println!("}}");
}

fn ptr_json(ptr: Option<u64>) -> String {
	match ptr {
		Some(value) => format!("\"0x{value:016x}\""),
		None => "null".to_owned(),
	}
}

fn str_json(value: Option<&str>) -> String {
	match value {
		Some(item) => format!("\"{item}\""),
		None => "null".to_owned(),
	}
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
