use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::blend::compression::decode_bytes;
use crate::blend::{BlendError, BlendHeader, Block, BlockIter, Compression, Dna, PointerIndex, Result};

pub struct BlendFile {
	pub header: BlendHeader,
	pub compression: Compression,
	bytes: Vec<u8>,
	blocks_offset: usize,
}

impl BlendFile {
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

	pub fn bytes(&self) -> &[u8] {
		&self.bytes
	}

	pub fn blocks(&self) -> BlockIter<'_> {
		BlockIter::new(&self.bytes, self.blocks_offset)
	}

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

	pub fn dna(&self) -> Result<Dna> {
		let block = self.find_first_block_by_code(*b"DNA1")?.ok_or(BlendError::DnaNotFound)?;
		Dna::parse(block.payload)
	}

	pub fn find_first_block_by_code(&self, code: [u8; 4]) -> Result<Option<Block<'_>>> {
		for block in self.blocks() {
			let block = block?;
			if block.head.code == code {
				return Ok(Some(block));
			}
		}
		Ok(None)
	}

	pub fn pointer_index(&self) -> Result<PointerIndex<'_>> {
		PointerIndex::build(self)
	}
}

pub struct BlockStats {
	pub block_count: u32,
	pub has_dna1: bool,
	pub has_endb: bool,
	pub last_code: [u8; 4],
	pub codes: HashMap<[u8; 4], u32>,
}
