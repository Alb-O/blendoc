use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, IdRecord, Result, scan_id_blocks};

/// Scan and print ID-root block summaries.
pub fn run(path: PathBuf, code: Option<String>, type_name: Option<String>, limit: Option<usize>, json: bool) -> Result<()> {
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

fn parse_block_code(code: &str) -> Result<[u8; 4]> {
	if code.is_empty() || code.len() > 4 || !code.is_ascii() {
		return Err(BlendError::InvalidBlockCode { code: code.to_owned() });
	}

	let mut out = [0_u8; 4];
	out[..code.len()].copy_from_slice(code.as_bytes());
	Ok(out)
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

fn ptr_json(value: Option<u64>) -> String {
	match value {
		Some(ptr) => format!("\"0x{ptr:016x}\""),
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
