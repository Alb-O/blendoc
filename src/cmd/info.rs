use std::path::PathBuf;

use blendoc::blend::{BlendFile, PointerStorage, Result};

#[derive(clap::Args)]
pub struct Args {
	pub path: PathBuf,
}

/// Print high-level file and block statistics.
pub fn run(args: Args) -> Result<()> {
	let Args { path } = args;

	let blend = BlendFile::open(&path)?;
	let stats = blend.scan_block_stats()?;
	let pointer_storage = blend.pointer_index()?.storage();

	println!("path: {}", path.display());
	println!("compression: {}", blend.compression.as_str());
	println!("header_size: {}", blend.header.header_size);
	println!("format_version: {}", blend.header.format_version);
	println!("version: {}", blend.header.version);
	println!("bhead_layout: large_bhead8");
	println!("endianness: little");
	println!("pointer_size: 8");
	println!("pointer_storage: {}", pointer_storage_label(pointer_storage));
	println!("block_count: {}", stats.block_count);
	println!("has_dna1: {}", stats.has_dna1);
	println!("has_endb: {}", stats.has_endb);
	println!("last_code: {}", code_label(stats.last_code));

	let mut entries: Vec<_> = stats.codes.into_iter().collect();
	entries.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

	println!("top_codes:");
	for (code, count) in entries.into_iter().take(12) {
		println!("  {}: {}", code_label(code), count);
	}

	Ok(())
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
