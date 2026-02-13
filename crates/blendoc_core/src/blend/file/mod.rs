use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::blend::compression::decode_bytes;
use crate::blend::{BlendError, BlendHeader, Block, BlockIter, Compression, Dna, PointerIndex, Result};

/// Opened blend container with decoded bytes and parsed header.
pub struct BlendFile {
	/// Parsed file header.
	pub header: BlendHeader,
	/// Compression mode detected for source bytes.
	pub compression: Compression,
	bytes: Vec<u8>,
	blocks_offset: usize,
}

impl BlendFile {
	/// Read, decode, and parse a blend file from disk.
	pub fn open(path: impl AsRef<Path>) -> Result<Self> {
		let raw = fs::read(path)?;
		let (compression, bytes) = decode_bytes(raw)?;
		let header = BlendHeader::parse(&bytes)?;
		if header.header_size > bytes.len() {
			return Err(BlendError::InvalidHeader);
		}

		Ok(Self {
			header,
			compression,
			bytes,
			blocks_offset: header.header_size,
		})
	}

	/// Return decoded raw bytes backing this file.
	pub fn bytes(&self) -> &[u8] {
		&self.bytes
	}

	/// Iterate all blocks starting at header-defined offset.
	pub fn blocks(&self) -> BlockIter<'_> {
		BlockIter::new(&self.bytes, self.blocks_offset, self.header)
	}

	/// Scan basic block distribution statistics.
	pub fn scan_block_stats(&self) -> Result<BlockStats> {
		let mut stats = BlockStats {
			block_count: 0,
			has_dna1: false,
			has_endb: false,
			last_code: [0_u8; 4],
			codes: HashMap::new(),
		};

		for block in self.blocks() {
			let block = block?;
			stats.block_count += 1;
			stats.last_code = block.head.code;
			*stats.codes.entry(block.head.code).or_insert(0) += 1;
			if block.head.code == *b"DNA1" {
				stats.has_dna1 = true;
			}
			if block.head.is_endb() {
				stats.has_endb = true;
			}
		}

		Ok(stats)
	}

	/// Parse and return the first `DNA1` block as SDNA tables.
	pub fn dna(&self) -> Result<Dna> {
		let block = self.find_first_block_by_code(*b"DNA1")?.ok_or(BlendError::DnaNotFound)?;
		Dna::parse(block.payload, self.header.endianness, self.header.pointer_size)
	}

	/// Find the first block matching a four-byte code.
	pub fn find_first_block_by_code(&self, code: [u8; 4]) -> Result<Option<Block<'_>>> {
		for block in self.blocks() {
			let block = block?;
			if block.head.code == code {
				return Ok(Some(block));
			}
		}
		Ok(None)
	}

	/// Build an index for old-pointer resolution.
	pub fn pointer_index(&self) -> Result<PointerIndex<'_>> {
		PointerIndex::build(self)
	}
}

/// Aggregate block-level counts from a linear scan.
pub struct BlockStats {
	/// Number of parsed blocks.
	pub block_count: u32,
	/// Whether a `DNA1` block was seen.
	pub has_dna1: bool,
	/// Whether an `ENDB` terminator block was seen.
	pub has_endb: bool,
	/// Code of the final block visited.
	pub last_code: [u8; 4],
	/// Frequency table by block code.
	pub codes: HashMap<[u8; 4], u32>,
}

#[cfg(test)]
mod tests;
