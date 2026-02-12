use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, IdIndex, XrefOptions, find_inbound_refs_to_ptr, scan_id_blocks};

/// Find inbound references to a selected target pointer.
pub fn run(
	path: PathBuf,
	id_name: Option<String>,
	ptr: Option<String>,
	refs_depth: Option<u32>,
	limit: Option<usize>,
	json: bool,
) -> blendoc::blend::Result<()> {
	let selector = parse_selector(id_name, ptr)?;

	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;
	let index = blend.pointer_index()?;
	let ids = IdIndex::build(scan_id_blocks(&blend, &dna)?);

	let (target_ptr, target_label) = match selector {
		TargetSelector::Id(name) => {
			let row = ids.get_by_name(&name).ok_or(BlendError::IdRecordNotFound { name: name.clone() })?;
			(row.old_ptr, format!("id:{}", row.id_name))
		}
		TargetSelector::Ptr(ptr) => (ptr, format!("ptr:0x{ptr:016x}")),
	};

	let typed = index
		.resolve_typed(&dna, target_ptr)
		.ok_or(BlendError::ChaseUnresolvedPtr { ptr: target_ptr })?;
	let element_index = typed.element_index.ok_or(BlendError::ChasePtrOutOfBounds { ptr: target_ptr })?;
	let offset = element_index
		.checked_mul(typed.struct_size)
		.ok_or(BlendError::ChasePtrOutOfBounds { ptr: target_ptr })?;
	let offset = u64::try_from(offset).map_err(|_| BlendError::ChasePtrOutOfBounds { ptr: target_ptr })?;
	let target_canonical = typed
		.base
		.entry
		.start_old
		.checked_add(offset)
		.ok_or(BlendError::ChasePtrOutOfBounds { ptr: target_ptr })?;
	let target_type = dna
		.struct_by_sdna(typed.base.entry.block.head.sdna_nr)
		.map(|item| dna.type_name(item.type_idx))
		.unwrap_or("<unknown>");
	let target_id = ids.get_by_ptr(target_canonical).map(|item| item.id_name.as_ref());

	let mut options = XrefOptions::default();
	if let Some(refs_depth) = refs_depth {
		options.ref_scan.max_depth = refs_depth;
	}
	if let Some(limit) = limit {
		options.max_results = limit;
	}

	let refs = find_inbound_refs_to_ptr(&dna, &index, &ids, target_ptr, &options)?;

	if json {
		print_json(&path, &target_label, target_canonical, target_type, target_id, &refs);
		return Ok(());
	}

	println!("path: {}", path.display());
	println!("target: {target_label}");
	println!("target_canonical: 0x{target_canonical:016x}");
	println!("target_type: {target_type}");
	println!("target_id: {}", target_id.unwrap_or("-"));
	println!("inbound: {}", refs.len());
	for inbound in refs {
		println!(
			"{}({}) -{}-> {}({})",
			inbound.from_id.as_deref().unwrap_or("-"),
			inbound.from_type,
			inbound.field,
			target_id.unwrap_or("-"),
			target_type
		);
	}

	Ok(())
}

enum TargetSelector {
	Id(String),
	Ptr(u64),
}

fn parse_selector(id_name: Option<String>, ptr: Option<String>) -> blendoc::blend::Result<TargetSelector> {
	let supplied = usize::from(id_name.is_some()) + usize::from(ptr.is_some());
	if supplied != 1 {
		return Err(BlendError::InvalidChaseRoot);
	}

	if let Some(id_name) = id_name {
		return Ok(TargetSelector::Id(id_name));
	}
	if let Some(ptr) = ptr {
		return Ok(TargetSelector::Ptr(parse_ptr(&ptr)?));
	}

	Err(BlendError::InvalidChaseRoot)
}

fn parse_ptr(value: &str) -> blendoc::blend::Result<u64> {
	let parsed = if let Some(stripped) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
		u64::from_str_radix(stripped, 16)
	} else {
		value.parse::<u64>()
	};

	parsed.map_err(|_| BlendError::InvalidPointerLiteral { value: value.to_owned() })
}

fn print_json(
	path: &std::path::Path,
	target_label: &str,
	target_canonical: u64,
	target_type: &str,
	target_id: Option<&str>,
	refs: &[blendoc::blend::InboundRef],
) {
	println!("{{");
	println!("  \"path\": \"{}\",", json_escape(&path.display().to_string()));
	println!("  \"target\": \"{}\",", json_escape(target_label));
	println!("  \"target_canonical\": \"0x{target_canonical:016x}\",");
	println!("  \"target_type\": \"{}\",", json_escape(target_type));
	println!("  \"target_id\": {},", str_json(target_id.map(json_escape).as_deref()));
	println!("  \"inbound\": [");
	for (idx, inbound) in refs.iter().enumerate() {
		let comma = if idx + 1 == refs.len() { "" } else { "," };
		println!(
			"    {{\"from\":\"0x{:016x}\",\"from_type\":\"{}\",\"from_id\":{},\"field\":\"{}\"}}{}",
			inbound.from,
			json_escape(&inbound.from_type),
			str_json(inbound.from_id.as_deref().map(json_escape).as_deref()),
			json_escape(&inbound.field),
			comma,
		);
	}
	println!("  ]");
	println!("}}");
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
