use std::path::PathBuf;

use blendoc::blend::DecodeOptions;

use crate::cmd::decode::run_with_code;
use crate::cmd::print::PrintOptions;

/// Decode and print the first `SC\0\0` scene block.
pub fn run(path: PathBuf) -> blendoc::blend::Result<()> {
	run_with_code(path, [b'S', b'C', 0, 0], DecodeOptions::for_scene_inspect(), PrintOptions::for_scene_inspect())
}
