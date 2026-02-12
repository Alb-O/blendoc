//! Shared test helpers for workspace crates.

use std::path::{Path, PathBuf};

/// Resolve the workspace root path.
pub fn workspace_root() -> PathBuf {
	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
	manifest_dir
		.join("..")
		.join("..")
		.canonicalize()
		.unwrap_or_else(|_| manifest_dir.join("..").join(".."))
}

/// Resolve a fixture path under `<workspace>/fixtures`.
pub fn fixture_path(name: &str) -> PathBuf {
	workspace_root().join("fixtures").join(name)
}

/// Resolve the workspace target directory.
pub fn target_dir() -> PathBuf {
	std::env::var_os("CARGO_TARGET_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| workspace_root().join("target"))
}
