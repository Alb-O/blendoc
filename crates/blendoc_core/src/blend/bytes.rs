use crate::blend::{BlendError, Result};

/// Simple bounded cursor over an immutable byte slice.
pub struct Cursor<'a> {
	bytes: &'a [u8],
	pos: usize,
}

impl<'a> Cursor<'a> {
	/// Create a cursor at position 0.
	pub fn new(bytes: &'a [u8]) -> Self {
		Self { bytes, pos: 0 }
	}

	/// Return current byte offset.
	pub fn pos(&self) -> usize {
		self.pos
	}

	/// Return remaining unread bytes.
	pub fn remaining(&self) -> usize {
		self.bytes.len().saturating_sub(self.pos)
	}

	/// Read exactly `n` bytes and advance cursor.
	pub fn read_exact(&mut self, n: usize) -> Result<&'a [u8]> {
		if n > self.remaining() {
			return Err(BlendError::UnexpectedEof {
				at: self.pos,
				need: n,
				rem: self.remaining(),
			});
		}

		let start = self.pos;
		self.pos += n;
		Ok(&self.bytes[start..self.pos])
	}

	/// Read a four-byte code.
	pub fn read_code4(&mut self) -> Result<[u8; 4]> {
		let raw = self.read_exact(4)?;
		let mut out = [0_u8; 4];
		out.copy_from_slice(raw);
		Ok(out)
	}

	/// Read a little-endian `u16`.
	pub fn read_u16_le(&mut self) -> Result<u16> {
		let raw = self.read_exact(2)?;
		let mut buf = [0_u8; 2];
		buf.copy_from_slice(raw);
		Ok(u16::from_le_bytes(buf))
	}

	/// Read a little-endian `u32`.
	pub fn read_u32_le(&mut self) -> Result<u32> {
		let raw = self.read_exact(4)?;
		let mut buf = [0_u8; 4];
		buf.copy_from_slice(raw);
		Ok(u32::from_le_bytes(buf))
	}

	/// Read a little-endian `u64`.
	pub fn read_u64_le(&mut self) -> Result<u64> {
		let raw = self.read_exact(8)?;
		let mut buf = [0_u8; 8];
		buf.copy_from_slice(raw);
		Ok(u64::from_le_bytes(buf))
	}

	/// Read a little-endian `i64`.
	pub fn read_i64_le(&mut self) -> Result<i64> {
		let raw = self.read_exact(8)?;
		let mut buf = [0_u8; 8];
		buf.copy_from_slice(raw);
		Ok(i64::from_le_bytes(buf))
	}

	/// Advance to the next 4-byte aligned position.
	pub fn align4(&mut self) -> Result<()> {
		let aligned = (self.pos + 3) & !3;
		let skip = aligned.saturating_sub(self.pos);
		let _ = self.read_exact(skip)?;
		Ok(())
	}

	/// Read a zero-terminated byte string without the terminator.
	pub fn read_cstring_bytes(&mut self) -> Result<&'a [u8]> {
		let start = self.pos;
		let rem = &self.bytes[self.pos..];
		let Some(rel_end) = rem.iter().position(|byte| *byte == 0) else {
			return Err(BlendError::UnexpectedEof {
				at: self.pos,
				need: 1,
				rem: self.remaining(),
			});
		};

		let end = start + rel_end;
		self.pos = end + 1;
		Ok(&self.bytes[start..end])
	}
}
