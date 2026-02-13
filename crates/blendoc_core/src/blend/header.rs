use crate::blend::{BlendError, Result};

/// Byte endianness marker stored in blend headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
	/// Little-endian byte order (`v` marker).
	Little,
	/// Big-endian byte order (`V` marker).
	Big,
}

impl Endianness {
	/// Stable lowercase label.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Little => "little",
			Self::Big => "big",
		}
	}
}

/// Parsed blend file header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendHeader {
	/// Total file header size in bytes.
	pub header_size: usize,
	/// Blend container format version (`0` for legacy headers, `1` for v1 headers).
	pub format_version: u16,
	/// Blender version encoded as decimal digits (for example `500` or `302`).
	pub version: u16,
	/// Pointer width in bytes.
	pub pointer_size: usize,
	/// File byte order.
	pub endianness: Endianness,
}

impl BlendHeader {
	/// Minimum number of bytes required for the modern v1 header prefix.
	pub const MIN_SIZE: usize = 17;
	/// Exact size of legacy headers (`BLENDER-v302` style).
	pub const LEGACY_SIZE: usize = 12;
	/// Synthetic format marker for legacy headers.
	pub const LEGACY_FORMAT_VERSION: u16 = 0;
	/// Modern v1 format marker.
	pub const V1_FORMAT_VERSION: u16 = 1;

	/// Parse a blend header from the beginning of `bytes`.
	pub fn parse(bytes: &[u8]) -> Result<Self> {
		let prefix = bytes.get(0..7).ok_or(BlendError::InvalidHeader)?;
		if prefix != b"BLENDER" {
			return Err(BlendError::InvalidHeader);
		}

		let kind = bytes.get(7).copied().ok_or(BlendError::InvalidHeader)?;
		if kind.is_ascii_digit() {
			return Self::parse_v1(bytes);
		}

		Self::parse_legacy(bytes)
	}

	/// Return human-readable block-header layout label.
	pub fn bhead_layout_label(self) -> &'static str {
		if self.format_version == Self::LEGACY_FORMAT_VERSION {
			"legacy"
		} else {
			"large_bhead8"
		}
	}

	fn parse_v1(bytes: &[u8]) -> Result<Self> {
		let header = bytes.get(0..Self::MIN_SIZE).ok_or(BlendError::InvalidHeader)?;

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
		if format_version != Self::V1_FORMAT_VERSION {
			return Err(BlendError::UnsupportedFormatVersion { version: format_version });
		}

		if header_size != 17 {
			return Err(BlendError::UnsupportedPointerSize { header_size });
		}
		let pointer_size = 8;

		let endianness = parse_endianness_marker(header[12]).ok_or(BlendError::InvalidHeader)?;
		let version = parse_digits(&header[13..17]).ok_or(BlendError::InvalidHeader)?;

		Ok(Self {
			header_size,
			format_version,
			version,
			pointer_size,
			endianness,
		})
	}

	fn parse_legacy(bytes: &[u8]) -> Result<Self> {
		let header = bytes.get(0..Self::LEGACY_SIZE).ok_or(BlendError::InvalidHeader)?;
		let pointer_size = match header[7] {
			b'_' => 4,
			b'-' => 8,
			_ => return Err(BlendError::InvalidHeader),
		};
		let endianness = parse_endianness_marker(header[8]).ok_or(BlendError::InvalidHeader)?;
		let version = parse_digits(&header[9..12]).ok_or(BlendError::InvalidHeader)?;

		Ok(Self {
			header_size: Self::LEGACY_SIZE,
			format_version: Self::LEGACY_FORMAT_VERSION,
			version,
			pointer_size,
			endianness,
		})
	}
}

fn parse_endianness_marker(byte: u8) -> Option<Endianness> {
	match byte {
		b'v' => Some(Endianness::Little),
		b'V' => Some(Endianness::Big),
		_ => None,
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

#[cfg(test)]
mod tests {
	use crate::blend::{BlendError, BlendHeader, Endianness};

	#[test]
	fn parses_large_bhead8_header() {
		let header = BlendHeader::parse(b"BLENDER17-01v0500").expect("header parses");
		assert_eq!(header.header_size, 17);
		assert_eq!(header.format_version, 1);
		assert_eq!(header.version, 500);
		assert_eq!(header.pointer_size, 8);
		assert_eq!(header.endianness, Endianness::Little);
		assert_eq!(header.bhead_layout_label(), "large_bhead8");
	}

	#[test]
	fn rejects_non_large_bhead8_header_size_marker() {
		let err = BlendHeader::parse(b"BLENDER18-01v0500X").expect_err("non-17 size marker should fail");
		assert!(matches!(err, BlendError::UnsupportedPointerSize { header_size: 18 }));
	}

	#[test]
	fn parses_legacy_little_endian_header() {
		let header = BlendHeader::parse(b"BLENDER-v302").expect("legacy header parses");
		assert_eq!(header.header_size, BlendHeader::LEGACY_SIZE);
		assert_eq!(header.format_version, BlendHeader::LEGACY_FORMAT_VERSION);
		assert_eq!(header.version, 302);
		assert_eq!(header.pointer_size, 8);
		assert_eq!(header.endianness, Endianness::Little);
		assert_eq!(header.bhead_layout_label(), "legacy");
	}

	#[test]
	fn parses_legacy_big_endian_header() {
		let header = BlendHeader::parse(b"BLENDER_V248").expect("legacy big-endian header parses");
		assert_eq!(header.header_size, BlendHeader::LEGACY_SIZE);
		assert_eq!(header.format_version, BlendHeader::LEGACY_FORMAT_VERSION);
		assert_eq!(header.version, 248);
		assert_eq!(header.pointer_size, 4);
		assert_eq!(header.endianness, Endianness::Big);
	}
}
