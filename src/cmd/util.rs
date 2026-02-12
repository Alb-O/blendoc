use blendoc::blend::{BlendError, Result};

/// Common selector form for roots that accept `--code`, `--ptr`, or `--id`.
pub(crate) enum RootSelector {
	Code([u8; 4]),
	Ptr(u64),
	Id(String),
}

/// Common selector form for targets that accept `--ptr` or `--id`.
pub(crate) enum IdOrPtrSelector {
	Ptr(u64),
	Id(String),
}

/// Parse up-to-4 ASCII block code into padded `[u8; 4]`.
pub(crate) fn parse_block_code(code: &str) -> Result<[u8; 4]> {
	if code.is_empty() || code.len() > 4 || !code.is_ascii() {
		return Err(BlendError::InvalidBlockCode { code: code.to_owned() });
	}

	let mut out = [0_u8; 4];
	out[..code.len()].copy_from_slice(code.as_bytes());
	Ok(out)
}

/// Parse decimal or `0x`-prefixed hex pointer literal.
pub(crate) fn parse_ptr(value: &str) -> Result<u64> {
	let parsed = if let Some(stripped) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
		u64::from_str_radix(stripped, 16)
	} else {
		value.parse::<u64>()
	};

	parsed.map_err(|_| BlendError::InvalidPointerLiteral { value: value.to_owned() })
}

/// Parse selector requiring exactly one of `--code`, `--ptr`, or `--id`.
pub(crate) fn parse_root_selector(code: Option<String>, ptr: Option<String>, id_name: Option<String>) -> Result<RootSelector> {
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

/// Parse selector requiring exactly one of `--ptr` or `--id`.
pub(crate) fn parse_id_or_ptr_selector(id_name: Option<String>, ptr: Option<String>) -> Result<IdOrPtrSelector> {
	let supplied = usize::from(id_name.is_some()) + usize::from(ptr.is_some());
	if supplied != 1 {
		return Err(BlendError::InvalidChaseRoot);
	}

	if let Some(id_name) = id_name {
		return Ok(IdOrPtrSelector::Id(id_name));
	}
	if let Some(ptr) = ptr {
		return Ok(IdOrPtrSelector::Ptr(parse_ptr(&ptr)?));
	}

	Err(BlendError::InvalidChaseRoot)
}

/// Render block code bytes as printable label.
pub(crate) fn render_code(code: [u8; 4]) -> String {
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

/// Escape text for embedding in JSON string values.
pub(crate) fn json_escape(input: &str) -> String {
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

/// Render optional string as JSON value.
pub(crate) fn str_json(value: Option<&str>) -> String {
	match value {
		Some(item) => format!("\"{item}\""),
		None => "null".to_owned(),
	}
}

/// Render optional pointer as JSON value.
pub(crate) fn ptr_json(value: Option<u64>) -> String {
	match value {
		Some(ptr) => format!("\"0x{ptr:016x}\""),
		None => "null".to_owned(),
	}
}

/// Escape text for Graphviz DOT label values.
pub(crate) fn dot_escape(input: &str) -> String {
	input.replace('\\', "\\\\").replace('"', "\\\"")
}
