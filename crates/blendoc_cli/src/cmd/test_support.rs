use std::path::PathBuf;
use std::process::Output;

use blendoc_testkit::{fixture_path as shared_fixture_path, run_blendoc as shared_run_blendoc, run_blendoc_json as shared_run_blendoc_json};

pub(crate) fn fixture_path(name: &str) -> PathBuf {
	shared_fixture_path(name)
}

pub(crate) fn run_blendoc(args: &[&str]) -> Output {
	shared_run_blendoc(args)
}

pub(crate) fn run_blendoc_json(args: &[&str]) -> serde_json::Value {
	shared_run_blendoc_json(args)
}
