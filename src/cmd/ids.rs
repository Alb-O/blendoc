use std::path::PathBuf;

use blendoc::blend::{BlendFile, IdRecord, Result, scan_id_blocks};

use crate::cmd::util::{json_escape, parse_block_code, ptr_json, render_code};

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
		Some(ptr) => format!("0x{ptr:016x}"),
		None => "-".to_owned(),
	}
}

fn print_json_rows(rows: &[IdRecord]) {
	println!("[");
	for (idx, row) in rows.iter().enumerate() {
		let comma = if idx + 1 == rows.len() { "" } else { "," };
		println!(
			"  {{\"old_ptr\":\"0x{:016x}\",\"code\":\"{}\",\"sdna_nr\":{},\"type\":\"{}\",\"id_name\":\"{}\",\"next\":{},\"prev\":{},\"lib\":{}}}{}",
			row.old_ptr,
			json_escape(&render_code(row.code)),
			row.sdna_nr,
			json_escape(&row.type_name),
			json_escape(&row.id_name),
			ptr_json(row.next),
			ptr_json(row.prev),
			ptr_json(row.lib),
			comma,
		);
	}
	println!("]");
}
