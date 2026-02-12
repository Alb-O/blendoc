//! Shared test helpers for workspace crates.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

static BLENDOC_BIN: OnceLock<PathBuf> = OnceLock::new();

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

/// Resolve the `blendoc` CLI binary path, building it when needed.
pub fn blendoc_bin() -> &'static PathBuf {
	BLENDOC_BIN.get_or_init(resolve_blendoc_bin)
}

/// Run the `blendoc` CLI with the provided args.
pub fn run_blendoc(args: &[&str]) -> Output {
	Command::new(blendoc_bin()).args(args).output().expect("blendoc command executes")
}

/// Run `blendoc` and parse stdout as JSON.
pub fn run_blendoc_json(args: &[&str]) -> serde_json::Value {
	let output = run_blendoc(args);
	assert!(
		output.status.success(),
		"blendoc command failed with status={}: {}",
		output.status,
		String::from_utf8_lossy(&output.stderr)
	);
	serde_json::from_slice(&output.stdout).expect("stdout should be valid json")
}

fn resolve_blendoc_bin() -> PathBuf {
	if let Ok(path) = std::env::var("CARGO_BIN_EXE_blendoc") {
		return PathBuf::from(path);
	}

	let workspace = workspace_root();
	let target = target_dir();
	let mut bin = target.join("debug");
	bin.push(if cfg!(windows) { "blendoc.exe" } else { "blendoc" });

	let status = Command::new("cargo")
		.current_dir(&workspace)
		.args(["build", "--quiet", "-p", "blendoc_cli", "--bin", "blendoc"])
		.status()
		.expect("cargo build executes");
	assert!(status.success(), "failed to build blendoc binary at {}", bin.display());

	bin
}
