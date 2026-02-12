use std::path::PathBuf;

use blendoc::blend::{BlendFile, PointerIndex, PointerStorage, Result};

use crate::cmd::util::{emit_json, ptr_hex};

#[derive(clap::Args)]
pub struct Args {
	pub path: PathBuf,
	#[arg(long)]
	pub json: bool,
}

/// Print high-level file and block statistics.
pub fn run(args: Args) -> Result<()> {
	let Args { path, json } = args;

	let blend = BlendFile::open(&path)?;
	let stats = blend.scan_block_stats()?;
	let pointer_index = blend.pointer_index()?;
	let pointer_storage = pointer_index.storage();
	let pointer_diag = analyze_pointer_index(&pointer_index);

	let mut entries: Vec<_> = stats.codes.into_iter().collect();
	entries.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

	if json {
		let payload = InfoJson {
			path: path.display().to_string(),
			compression: blend.compression.as_str().to_owned(),
			header_size: blend.header.header_size,
			format_version: blend.header.format_version,
			version: blend.header.version,
			bhead_layout: "large_bhead8",
			endianness: "little",
			pointer_size: 8,
			pointer_storage: pointer_storage_label(pointer_storage).to_owned(),
			pointer_diagnostics: PointerDiagnosticsJson {
				indexed_entries: pointer_diag.indexed_entries,
				overlapping_ranges: pointer_diag.overlapping_ranges,
				duplicate_starts: pointer_diag.duplicate_starts,
				min_old: pointer_diag.min_old.map(ptr_hex),
				max_old: pointer_diag.max_old.map(ptr_hex),
				max_end: pointer_diag.max_end.map(ptr_hex),
			},
			block_count: stats.block_count,
			has_dna1: stats.has_dna1,
			has_endb: stats.has_endb,
			last_code: code_label(stats.last_code),
			top_codes: entries
				.iter()
				.take(12)
				.map(|(code, count)| CodeCountJson {
					code: code_label(*code),
					count: *count,
				})
				.collect(),
		};
		emit_json(&payload);
		return Ok(());
	}

	println!("path: {}", path.display());
	println!("compression: {}", blend.compression.as_str());
	println!("header_size: {}", blend.header.header_size);
	println!("format_version: {}", blend.header.format_version);
	println!("version: {}", blend.header.version);
	println!("bhead_layout: large_bhead8");
	println!("endianness: little");
	println!("pointer_size: 8");
	println!("pointer_storage: {}", pointer_storage_label(pointer_storage));
	println!("pointer_blocks_indexed: {}", pointer_diag.indexed_entries);
	println!("pointer_overlapping_ranges: {}", pointer_diag.overlapping_ranges);
	println!("pointer_duplicate_starts: {}", pointer_diag.duplicate_starts);
	println!("pointer_min_old: {}", ptr_hex_opt(pointer_diag.min_old));
	println!("pointer_max_old: {}", ptr_hex_opt(pointer_diag.max_old));
	println!("pointer_max_end: {}", ptr_hex_opt(pointer_diag.max_end));
	println!("block_count: {}", stats.block_count);
	println!("has_dna1: {}", stats.has_dna1);
	println!("has_endb: {}", stats.has_endb);
	println!("last_code: {}", code_label(stats.last_code));

	println!("top_codes:");
	for (code, count) in entries.into_iter().take(12) {
		println!("  {}: {}", code_label(code), count);
	}

	Ok(())
}

#[derive(Debug, Clone, Copy)]
struct PointerDiagnostics {
	indexed_entries: usize,
	overlapping_ranges: usize,
	duplicate_starts: usize,
	min_old: Option<u64>,
	max_old: Option<u64>,
	max_end: Option<u64>,
}

fn analyze_pointer_index(index: &PointerIndex<'_>) -> PointerDiagnostics {
	let entries = index.entries();
	if entries.is_empty() {
		return PointerDiagnostics {
			indexed_entries: 0,
			overlapping_ranges: 0,
			duplicate_starts: 0,
			min_old: None,
			max_old: None,
			max_end: None,
		};
	}

	let mut overlaps = 0_usize;
	let mut duplicate_starts = 0_usize;
	let mut max_end = 0_u64;
	let mut prev_start = None;

	for entry in entries {
		if entry.start_old < max_end {
			overlaps += 1;
		}
		max_end = max_end.max(entry.end_old);

		if prev_start == Some(entry.start_old) {
			duplicate_starts += 1;
		}
		prev_start = Some(entry.start_old);
	}

	PointerDiagnostics {
		indexed_entries: entries.len(),
		overlapping_ranges: overlaps,
		duplicate_starts,
		min_old: entries.first().map(|item| item.start_old),
		max_old: entries.last().map(|item| item.start_old),
		max_end: Some(max_end),
	}
}

fn ptr_hex_opt(value: Option<u64>) -> String {
	value.map(ptr_hex).unwrap_or_else(|| "-".to_owned())
}

#[derive(serde::Serialize)]
struct PointerDiagnosticsJson {
	indexed_entries: usize,
	overlapping_ranges: usize,
	duplicate_starts: usize,
	min_old: Option<String>,
	max_old: Option<String>,
	max_end: Option<String>,
}

#[derive(serde::Serialize)]
struct CodeCountJson {
	code: String,
	count: u32,
}

#[derive(serde::Serialize)]
struct InfoJson {
	path: String,
	compression: String,
	header_size: usize,
	format_version: u16,
	version: u16,
	bhead_layout: &'static str,
	endianness: &'static str,
	pointer_size: u8,
	pointer_storage: String,
	pointer_diagnostics: PointerDiagnosticsJson,
	block_count: u32,
	has_dna1: bool,
	has_endb: bool,
	last_code: String,
	top_codes: Vec<CodeCountJson>,
}

fn code_label(code: [u8; 4]) -> String {
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

fn pointer_storage_label(storage: PointerStorage) -> &'static str {
	match storage {
		PointerStorage::AddressRanges => "address_ranges",
		PointerStorage::StableIds => "stable_ids",
	}
}

#[cfg(test)]
mod tests;
