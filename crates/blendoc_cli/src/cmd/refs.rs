use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, IdIndex, RefRecord, RefScanOptions, scan_id_blocks, scan_id_link_provenance, scan_refs_from_ptr};

use crate::cmd::util::{RootSelector, emit_json, parse_root_selector, ptr_hex, render_code};

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

	let root_link = if json {
		let links = scan_id_link_provenance(&blend, &dna)?;
		let canonical = index.canonical_ptr(&dna, root_ptr).unwrap_or(root_ptr);
		links
			.iter()
			.find(|item| item.id_ptr == canonical)
			.map(|item| (item.linked, item.confidence.as_str()))
	} else {
		None
	};

	if json {
		print_json(&path, &root_label, root_ptr, &refs, root_link);
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

fn print_json(path: &std::path::Path, root_label: &str, root_ptr: u64, refs: &[RefRecord], root_link: Option<(bool, &str)>) {
	let payload = RefsJson {
		path: path.display().to_string(),
		root: root_label.to_owned(),
		root_ptr: ptr_hex(root_ptr),
		owner_linked: root_link.map(|item| item.0),
		owner_link_confidence: root_link.map(|item| item.1.to_owned()),
		refs: refs
			.iter()
			.map(|record| {
				if let Some(target) = &record.resolved {
					RefJson {
						field: record.field.to_string(),
						ptr: ptr_hex(record.ptr),
						canonical: Some(ptr_hex(target.canonical)),
						code: Some(render_code(target.code)),
						sdna_nr: Some(target.sdna_nr),
						type_name: Some(target.type_name.to_string()),
						id: target.id_name.as_deref().map(|item| item.to_string()),
					}
				} else {
					RefJson {
						field: record.field.to_string(),
						ptr: ptr_hex(record.ptr),
						canonical: None,
						code: None,
						sdna_nr: None,
						type_name: None,
						id: None,
					}
				}
			})
			.collect(),
	};

	emit_json(&payload);
}

#[derive(serde::Serialize)]
struct RefsJson {
	path: String,
	root: String,
	root_ptr: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	owner_linked: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	owner_link_confidence: Option<String>,
	refs: Vec<RefJson>,
}

#[derive(serde::Serialize)]
struct RefJson {
	field: String,
	ptr: String,
	canonical: Option<String>,
	code: Option<String>,
	sdna_nr: Option<u32>,
	#[serde(rename = "type")]
	type_name: Option<String>,
	id: Option<String>,
}
