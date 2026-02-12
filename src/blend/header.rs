use crate::blend::{BlendError, Result};

/// Parsed Blender 5+ file header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendHeader {
	/// Total file header size in bytes.
	pub header_size: usize,
	/// Blend container format version.
	pub format_version: u16,
	/// Blender version encoded as decimal digits (for example `500`).
	pub version: u16,
}

impl BlendHeader {
	/// Minimum number of bytes required for the modern header prefix.
	pub const MIN_SIZE: usize = 17;

	/// Parse a Blender 5+ header from the beginning of `bytes`.
	pub fn parse(bytes: &[u8]) -> Result<Self> {
		let header = bytes.get(0..Self::MIN_SIZE).ok_or(BlendError::InvalidHeader)?;
		if &header[0..7] != b"BLENDER" {
			return Err(BlendError::InvalidHeader);
		}

		let header_size = parse_digits(&header[7..9]).ok_or(BlendError::InvalidHeader)? as usize;
		if header_size < Self::MIN_SIZE {
			return Err(BlendError::InvalidHeader);
		}

		if bytes.len() < header_size {
			return Err(BlendError::UnexpectedEof {
				at: bytes.len(),
				need: header_size - bytes.len(),
				rem: 0,
			});
		}

		if header[9] != b'-' {
			return Err(BlendError::InvalidHeader);
		}

		let format_version = parse_digits(&header[10..12]).ok_or(BlendError::InvalidHeader)?;
		if format_version != 1 {
			return Err(BlendError::UnsupportedFormatVersion { version: format_version });
		}

		if header[12] != b'v' {
			return Err(BlendError::BigEndianUnsupported);
		}

		let version = parse_digits(&header[13..17]).ok_or(BlendError::InvalidHeader)?;
		if version < 500 {
			return Err(BlendError::UnsupportedBlendVersion { version });
		}

		Ok(Self {
			header_size,
			format_version,
			version,
		})
	}
}

fn parse_digits(bytes: &[u8]) -> Option<u16> {
	if bytes.is_empty() {
		return None;
	}

	let mut value = 0_u16;
	for byte in bytes {
		if !byte.is_ascii_digit() {
			return None;
		}
		value = value * 10 + u16::from(*byte - b'0');
	}
	Some(value)
}
