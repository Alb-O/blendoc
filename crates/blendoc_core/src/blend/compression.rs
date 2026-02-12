use std::io::Read;

use crate::blend::{BlendError, Result};

const BLEND_MAGIC: &[u8] = b"BLENDER";
const MAX_DECOMPRESSED_BYTES: usize = 512 * 1024 * 1024;
/// zstd frame magic used by compressed `.blend` files.
pub const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// Compression mode detected for a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
	/// Raw uncompressed stream.
	None,
	/// zstd-compressed stream.
	Zstd,
}

impl Compression {
	/// Render compression mode as a stable lowercase label.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Zstd => "zstd",
		}
	}
}

/// Detect and decode compression, returning `(mode, decoded_bytes)`.
pub fn decode_bytes(raw: Vec<u8>) -> Result<(Compression, Vec<u8>)> {
	if raw.starts_with(BLEND_MAGIC) {
		return Ok((Compression::None, raw));
	}

	if raw.starts_with(&ZSTD_MAGIC) {
		let out = decode_zstd(&raw)?;
		return Ok((Compression::Zstd, out));
	}

	Err(BlendError::UnknownMagic { magic: first4(&raw) })
}

fn decode_zstd(raw: &[u8]) -> Result<Vec<u8>> {
	let mut decoder = zstd::stream::read::Decoder::new(raw)?;
	let mut out = Vec::new();
	let mut buf = [0_u8; 8192];

	loop {
		let read = decoder.read(&mut buf)?;
		if read == 0 {
			break;
		}

		if out.len() + read > MAX_DECOMPRESSED_BYTES {
			return Err(BlendError::DecompressedTooLarge { limit: MAX_DECOMPRESSED_BYTES });
		}

		out.extend_from_slice(&buf[..read]);
	}

	if !out.starts_with(BLEND_MAGIC) {
		return Err(BlendError::NotBlendAfterDecompress);
	}

	Ok(out)
}

fn first4(bytes: &[u8]) -> [u8; 4] {
	let mut magic = [0_u8; 4];
	let take = bytes.len().min(4);
	magic[..take].copy_from_slice(&bytes[..take]);
	magic
}
