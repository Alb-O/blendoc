use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

static BLENDOC_BIN: OnceLock<PathBuf> = OnceLock::new();

pub(crate) fn fixture_path(name: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}

pub(crate) fn run_blendoc(args: &[&str]) -> Output {
	Command::new(blendoc_bin()).args(args).output().expect("blendoc command executes")
}

pub(crate) fn run_blendoc_json(args: &[&str]) -> serde_json::Value {
	let output = run_blendoc(args);
	assert!(
		output.status.success(),
		"blendoc command failed with status={}: {}",
		output.status,
		String::from_utf8_lossy(&output.stderr)
	);
	serde_json::from_slice(&output.stdout).expect("stdout should be valid json")
}

fn blendoc_bin() -> &'static PathBuf {
	BLENDOC_BIN.get_or_init(resolve_blendoc_bin)
}

fn resolve_blendoc_bin() -> PathBuf {
	if let Ok(path) = std::env::var("CARGO_BIN_EXE_blendoc") {
		return PathBuf::from(path);
	}

	let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let target_dir = std::env::var_os("CARGO_TARGET_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| manifest_dir.join("target"));

	let mut bin = target_dir.join("debug");
	bin.push(if cfg!(windows) { "blendoc.exe" } else { "blendoc" });

	let status = Command::new("cargo")
		.current_dir(&manifest_dir)
		.args(["build", "--quiet", "--bin", "blendoc"])
		.status()
		.expect("cargo build executes");
	assert!(status.success(), "failed to build blendoc binary at {}", bin.display());

	bin
}
