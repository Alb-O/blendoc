use std::path::PathBuf;

use blendoc::blend::{BlendFile, IdRecord, Result, scan_id_blocks};

use crate::cmd::util::{emit_json, parse_block_code, ptr_hex, ptr_hex_opt, render_code};

#[derive(clap::Args)]
pub struct Args {
	pub path: PathBuf,
	#[arg(long)]
	pub code: Option<String>,
	#[arg(long = "type")]
	pub type_name: Option<String>,
	#[arg(long)]
	pub limit: Option<usize>,
	#[arg(long)]
	pub json: bool,
}

/// Scan and print ID-root block summaries.
pub fn run(args: Args) -> Result<()> {
	let Args {
		path,
		code,
		type_name,
		limit,
		json,
	} = args;

	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;

	let mut rows = scan_id_blocks(&blend, &dna)?;

	if let Some(filter) = code {
		let parsed = parse_block_code(&filter)?;
		rows.retain(|row| row.code == parsed);
	}

	if let Some(filter) = type_name {
		rows.retain(|row| row.type_name.as_ref() == filter.as_str());
	}

	rows.sort_by_key(|row| row.old_ptr);

	if let Some(max) = limit {
		rows.truncate(max);
	}

	if json {
		print_json_rows(&rows);
		return Ok(());
	}

	println!("path: {}", path.display());
	println!("ids: {}", rows.len());
	println!("old_ptr\tcode\tsdna\ttype\tid_name\tnext\tprev\tlib");
	for row in rows {
		println!(
			"0x{:016x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
			row.old_ptr,
			render_code(row.code),
			row.sdna_nr,
			row.type_name,
			row.id_name,
			format_ptr(row.next),
			format_ptr(row.prev),
			format_ptr(row.lib)
		);
	}

	Ok(())
}

fn format_ptr(value: Option<u64>) -> String {
	match value {
		Some(ptr) => ptr_hex(ptr),
		None => "-".to_owned(),
	}
}

#[derive(serde::Serialize)]
struct IdRowJson {
	old_ptr: String,
	code: String,
	sdna_nr: u32,
	#[serde(rename = "type")]
	type_name: String,
	id_name: String,
	next: Option<String>,
	prev: Option<String>,
	lib: Option<String>,
}

fn print_json_rows(rows: &[IdRecord]) {
	let body: Vec<IdRowJson> = rows
		.iter()
		.map(|row| IdRowJson {
			old_ptr: ptr_hex(row.old_ptr),
			code: render_code(row.code),
			sdna_nr: row.sdna_nr,
			type_name: row.type_name.to_string(),
			id_name: row.id_name.to_string(),
			next: ptr_hex_opt(row.next),
			prev: ptr_hex_opt(row.prev),
			lib: ptr_hex_opt(row.lib),
		})
		.collect();

	emit_json(&body);
}
