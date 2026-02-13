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
mod tests;
