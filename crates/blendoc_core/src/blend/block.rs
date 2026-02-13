use crate::blend::bytes::Cursor;
use crate::blend::{BHead, BlendError, BlendHeader, Result};

/// Borrowed view over one parsed blend block.
#[derive(Debug, Clone, Copy)]
pub struct Block<'a> {
	/// Parsed block header.
	pub head: BHead,
	/// Raw payload bytes.
	pub payload: &'a [u8],
	/// Absolute byte offset where this block header starts.
	pub file_offset: usize,
}

/// Iterator over contiguous block records.
pub struct BlockIter<'a> {
	cursor: Cursor<'a>,
	offset_base: usize,
	header: BlendHeader,
	done: bool,
}

impl<'a> BlockIter<'a> {
	/// Create a block iterator starting at `offset`.
	pub fn new(bytes: &'a [u8], offset: usize, header: BlendHeader) -> Self {
		let slice = bytes.get(offset..).unwrap_or(&[]);
		Self {
			cursor: Cursor::new(slice),
			offset_base: offset,
			header,
			done: false,
		}
	}
}

impl<'a> Iterator for BlockIter<'a> {
	type Item = Result<Block<'a>>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.done {
			return None;
		}

		if self.cursor.remaining() == 0 {
			self.done = true;
			return None;
		}

		let file_offset = self.offset_base + self.cursor.pos();
		let head = match BHead::parse(&mut self.cursor, self.header) {
			Ok(value) => value,
			Err(err) => {
				self.done = true;
				return Some(Err(err));
			}
		};

		let payload_len = match usize::try_from(head.len) {
			Ok(value) => value,
			Err(_) => {
				self.done = true;
				return Some(Err(BlendError::BlockLenOutOfRange {
					at: file_offset,
					len: head.len,
					rem: self.cursor.remaining(),
				}));
			}
		};

		let rem = self.cursor.remaining();
		if payload_len > rem {
			self.done = true;
			return Some(Err(BlendError::BlockLenOutOfRange {
				at: file_offset,
				len: head.len,
				rem,
			}));
		}

		let payload = match self.cursor.read_exact(payload_len) {
			Ok(value) => value,
			Err(err) => {
				self.done = true;
				return Some(Err(err));
			}
		};

		if head.is_endb() {
			self.done = true;
		}

		Some(Ok(Block { head, payload, file_offset }))
	}
}
