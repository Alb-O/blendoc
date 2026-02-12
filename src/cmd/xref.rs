use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, IdIndex, XrefOptions, find_inbound_refs_to_ptr, scan_id_blocks};

use crate::cmd::util::{IdOrPtrSelector, json_escape, parse_id_or_ptr_selector, str_json};

#[derive(clap::Args)]
pub struct Args {
	pub file: PathBuf,
	#[arg(long = "id")]
	pub id_name: Option<String>,
	#[arg(long)]
	pub ptr: Option<String>,
	#[arg(long = "refs-depth")]
	pub refs_depth: Option<u32>,
	#[arg(long)]
	pub limit: Option<usize>,
	#[arg(long)]
	pub json: bool,
}

/// Find inbound references to a selected target pointer.
pub fn run(args: Args) -> blendoc::blend::Result<()> {
	let Args {
		file: path,
		id_name,
		ptr,
		refs_depth,
		limit,
		json,
	} = args;

	let selector = parse_id_or_ptr_selector(id_name, ptr)?;

	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;
	let index = blend.pointer_index()?;
	let ids = IdIndex::build(scan_id_blocks(&blend, &dna)?);

	let (target_ptr, target_label) = match selector {
		IdOrPtrSelector::Id(name) => {
			let row = ids.get_by_name(&name).ok_or(BlendError::IdRecordNotFound { name: name.clone() })?;
			(row.old_ptr, format!("id:{}", row.id_name))
		}
		IdOrPtrSelector::Ptr(ptr) => (ptr, format!("ptr:0x{ptr:016x}")),
	};

	let (target_canonical, typed) = index.resolve_canonical_typed(&dna, target_ptr)?;
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
