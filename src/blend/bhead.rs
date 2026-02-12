use crate::blend::bytes::Cursor;
use crate::blend::{BlendError, Result};

#[derive(Debug, Clone, Copy)]
pub struct BHead {
	pub code: [u8; 4],
	pub sdna_nr: u32,
	pub old: u64,
	pub len: u64,
	pub nr: u64,
}

impl BHead {
	pub fn parse(cursor: &mut Cursor<'_>) -> Result<Self> {
		let code = cursor.read_code4()?;
		let sdna_nr = cursor.read_u32_le()?;
		let old = cursor.read_u64_le()?;

		let len = cursor.read_i64_le()?;
		if len < 0 {
			return Err(BlendError::NegativeBlockLength { len });
		}

		let nr = cursor.read_i64_le()?;
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

	pub fn is_endb(&self) -> bool {
		self.code == *b"ENDB"
	}
}
