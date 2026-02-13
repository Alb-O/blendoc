use crate::blend::bytes::Cursor;
use crate::blend::{BlendError, BlendHeader, Result};

/// Parsed `LargeBHead8` block header.
#[derive(Debug, Clone, Copy)]
pub struct BHead {
	/// Four-byte block code.
	pub code: [u8; 4],
	/// SDNA struct index for payload interpretation.
	pub sdna_nr: u32,
	/// Stored address identifier used for pointer relocation.
	///
	/// In modern files this can be a stable opaque ID instead of a raw runtime pointer.
	pub old: u64,
	/// Payload byte length.
	pub len: u64,
	/// Number of elements stored in payload.
	pub nr: u64,
}

impl BHead {
	/// Parse a block header from cursor position.
	pub fn parse(cursor: &mut Cursor<'_>, header: BlendHeader) -> Result<Self> {
		if header.format_version == BlendHeader::LEGACY_FORMAT_VERSION {
			return Self::parse_legacy(cursor, header);
		}
		Self::parse_v1(cursor, header)
	}

	fn parse_v1(cursor: &mut Cursor<'_>, header: BlendHeader) -> Result<Self> {
		let code = cursor.read_code4()?;
		let sdna_nr = cursor.read_u32(header.endianness)?;
		let old = cursor.read_u64(header.endianness)?;

		let len = cursor.read_i64(header.endianness)?;
		if len < 0 {
			return Err(BlendError::NegativeBlockLength { len });
		}

		let nr = cursor.read_i64(header.endianness)?;
		if nr < 0 {
			return Err(BlendError::NegativeBlockCount { nr });
		}

		Ok(Self {
			code,
			sdna_nr,
			old,
			len: len as u64,
			nr: nr as u64,
		})
	}

	fn parse_legacy(cursor: &mut Cursor<'_>, header: BlendHeader) -> Result<Self> {
		let code = cursor.read_code4()?;
		let len = i64::from(cursor.read_i32(header.endianness)?);
		if len < 0 {
			return Err(BlendError::NegativeBlockLength { len });
		}

		let old = cursor.read_ptr(header.pointer_size, header.endianness)?;
		let sdna_nr = cursor.read_u32(header.endianness)?;
		let nr = i64::from(cursor.read_i32(header.endianness)?);
		if nr < 0 {
			return Err(BlendError::NegativeBlockCount { nr });
		}

		Ok(Self {
			code,
			sdna_nr,
			old,
			len: len as u64,
			nr: nr as u64,
		})
	}

	/// Return `true` when this is the terminal `ENDB` block.
	pub fn is_endb(&self) -> bool {
		self.code == *b"ENDB"
	}
}

#[cfg(test)]
mod tests {
	use crate::blend::bytes::Cursor;
	use crate::blend::{BHead, BlendHeader};

	#[test]
	fn parses_v1_little_endian_bhead() {
		let header = BlendHeader::parse(b"BLENDER17-01v0500").expect("header parses");
		let mut bytes = Vec::new();
		bytes.extend_from_slice(b"TEST");
		bytes.extend_from_slice(&3_u32.to_le_bytes());
		bytes.extend_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
		bytes.extend_from_slice(&16_i64.to_le_bytes());
		bytes.extend_from_slice(&2_i64.to_le_bytes());

		let mut cursor = Cursor::new(&bytes);
		let head = BHead::parse(&mut cursor, header).expect("bhead parses");
		assert_eq!(head.code, *b"TEST");
		assert_eq!(head.sdna_nr, 3);
		assert_eq!(head.old, 0x1122_3344_5566_7788);
		assert_eq!(head.len, 16);
		assert_eq!(head.nr, 2);
	}

	#[test]
	fn parses_legacy_big_endian_bhead() {
		let header = BlendHeader::parse(b"BLENDER_V248").expect("header parses");
		let mut bytes = Vec::new();
		bytes.extend_from_slice(b"TEST");
		bytes.extend_from_slice(&12_i32.to_be_bytes());
		bytes.extend_from_slice(&0x99AA_BBCC_u32.to_be_bytes());
		bytes.extend_from_slice(&7_u32.to_be_bytes());
		bytes.extend_from_slice(&1_i32.to_be_bytes());

		let mut cursor = Cursor::new(&bytes);
		let head = BHead::parse(&mut cursor, header).expect("legacy bhead parses");
		assert_eq!(head.code, *b"TEST");
		assert_eq!(head.sdna_nr, 7);
		assert_eq!(head.old, 0x99AA_BBCC);
		assert_eq!(head.len, 12);
		assert_eq!(head.nr, 1);
	}
}
