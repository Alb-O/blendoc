use blendoc::blend::{BlendError, Result};

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
