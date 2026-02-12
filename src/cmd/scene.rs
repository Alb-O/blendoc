use std::path::PathBuf;

use blendoc::blend::DecodeOptions;

use crate::cmd::decode::{PrintOptions, run_with_code};

pub fn run(path: PathBuf) -> blendoc::blend::Result<()> {
	run_with_code(path, [b'S', b'C', 0, 0], DecodeOptions::for_scene_inspect(), PrintOptions::for_scene_inspect())
}
