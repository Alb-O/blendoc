use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, DecodeOptions, decode_block_instances};

use crate::cmd::print::{PrintOptions, print_value};
use crate::cmd::util::{parse_block_code, render_code};

/// Decode and print the first block matching `code`.
pub fn run(path: PathBuf, code: String) -> blendoc::blend::Result<()> {
	let block_code = parse_block_code(&code)?;
	run_with_code(path, block_code, DecodeOptions::default(), PrintOptions::default())
}

/// Decode and print the first block matching a binary block code.
pub fn run_with_code(path: PathBuf, block_code: [u8; 4], decode_options: DecodeOptions, print_options: PrintOptions) -> blendoc::blend::Result<()> {
	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;
	let block = blend
		.find_first_block_by_code(block_code)?
		.ok_or(BlendError::BlockNotFound { code: block_code })?;
	let value = decode_block_instances(&dna, &block, &decode_options)?;

	println!("path: {}", path.display());
	println!("code: {}", render_code(block_code));
	println!("sdna_nr: {}", block.head.sdna_nr);
	println!("nr: {}", block.head.nr);
	println!("len: {}", block.head.len);
	println!("decoded:");
	print_value(&value, 0, 0, print_options, None, 0);

	Ok(())
}
