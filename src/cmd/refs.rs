use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, IdIndex, RefRecord, RefScanOptions, scan_id_blocks, scan_refs_from_ptr};

use crate::cmd::util::{RootSelector, json_escape, parse_root_selector, render_code, str_json};

#[derive(clap::Args)]
pub struct Args {
	pub file: PathBuf,
	#[arg(long)]
	pub code: Option<String>,
	#[arg(long)]
	pub ptr: Option<String>,
	#[arg(long = "id")]
	pub id_name: Option<String>,
	#[arg(long)]
	pub depth: Option<u32>,
	#[arg(long)]
	pub limit: Option<usize>,
	#[arg(long)]
	pub json: bool,
}

/// Scan and print pointer references from one selected root struct.
pub fn run(args: Args) -> blendoc::blend::Result<()> {
	let Args {
		file: path,
		code,
		ptr,
		id_name,
		depth,
		limit,
		json,
	} = args;

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
		RootSelector::Id(id_name) => {
			let row = ids.get_by_name(&id_name).ok_or(BlendError::IdRecordNotFound { name: id_name.clone() })?;
			(row.old_ptr, format!("id:{}", row.id_name))
		}
	};

	let mut options = RefScanOptions::default();
	if let Some(depth) = depth {
		options.max_depth = depth;
	}

	let mut refs = scan_refs_from_ptr(&dna, &index, &ids, root_ptr, &options)?;
	if let Some(max) = limit {
		refs.truncate(max);
	}

	if json {
		print_json(&path, &root_label, root_ptr, &refs);
		return Ok(());
	}

	println!("path: {}", path.display());
	println!("root: {root_label}");
	println!("root_ptr: 0x{root_ptr:016x}");
	println!("refs: {}", refs.len());
	println!("field\tptr\tcanonical\tcode\tsdna\ttype\tid");
	for record in refs {
		if let Some(target) = record.resolved {
			println!(
				"{}\t0x{:016x}\t0x{:016x}\t{}\t{}\t{}\t{}",
				record.field,
				record.ptr,
				target.canonical,
				render_code(target.code),
				target.sdna_nr,
				target.type_name,
				target.id_name.as_deref().unwrap_or("-")
			);
		} else {
			println!("{}\t0x{:016x}\t-\t-\t-\t-\t-", record.field, record.ptr);
		}
	}

	Ok(())
}

fn print_json(path: &std::path::Path, root_label: &str, root_ptr: u64, refs: &[RefRecord]) {
	println!("{{");
	println!("  \"path\": \"{}\",", json_escape(&path.display().to_string()));
	println!("  \"root\": \"{}\",", json_escape(root_label));
	println!("  \"root_ptr\": \"0x{root_ptr:016x}\",");
	println!("  \"refs\": [");
	for (idx, record) in refs.iter().enumerate() {
		let comma = if idx + 1 == refs.len() { "" } else { "," };
		if let Some(target) = &record.resolved {
			println!(
				"    {{\"field\":\"{}\",\"ptr\":\"0x{:016x}\",\"canonical\":\"0x{:016x}\",\"code\":\"{}\",\"sdna_nr\":{},\"type\":\"{}\",\"id\":{}}}{}",
				json_escape(&record.field),
				record.ptr,
				target.canonical,
				json_escape(&render_code(target.code)),
				target.sdna_nr,
				json_escape(&target.type_name),
				str_json(target.id_name.as_deref().map(json_escape).as_deref()),
				comma,
			);
		} else {
			println!(
				"    {{\"field\":\"{}\",\"ptr\":\"0x{:016x}\",\"canonical\":null,\"code\":null,\"sdna_nr\":null,\"type\":null,\"id\":null}}{}",
				json_escape(&record.field),
				record.ptr,
				comma,
			);
		}
	}
	println!("  ]");
	println!("}}");
}
