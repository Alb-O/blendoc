#![allow(missing_docs)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

#[test]
fn show_json_output_is_valid_and_structured() {
	let json = run_json(vec![
		"show".to_owned(),
		fixture_path("character.blend").display().to_string(),
		"--id".to_owned(),
		"WOWorld".to_owned(),
		"--json".to_owned(),
	]);

	assert_eq!(json["root"], "id:WOWorld");
	assert!(json["canonical"].as_str().is_some_and(|item| item.starts_with("0x")));
	assert!(json["value"].is_object(), "expected top-level value object");
}

#[test]
fn graph_json_output_contains_nodes_and_edges() {
	let json = run_json(vec![
		"graph".to_owned(),
		fixture_path("character.blend").display().to_string(),
		"--id".to_owned(),
		"SCScene".to_owned(),
		"--depth".to_owned(),
		"1".to_owned(),
		"--refs-depth".to_owned(),
		"1".to_owned(),
		"--json".to_owned(),
	]);

	assert_eq!(json["root"], "id:SCScene");
	assert!(json["nodes"].as_array().is_some_and(|items| !items.is_empty()), "expected graph nodes");
	assert!(json["edges"].as_array().is_some(), "expected graph edges array");
}

#[test]
fn route_json_output_includes_path_edges_array() {
	let json = run_json(vec![
		"route".to_owned(),
		fixture_path("character.blend").display().to_string(),
		"--from-id".to_owned(),
		"SCScene".to_owned(),
		"--to-id".to_owned(),
		"WOWorld".to_owned(),
		"--json".to_owned(),
	]);

	assert_eq!(json["from"]["selector"], "id:SCScene");
	assert_eq!(json["to"]["selector"], "id:WOWorld");
	assert!(json["path_edges"].is_array(), "expected path_edges array");
}

fn run_json(args: Vec<String>) -> Value {
	let output = Command::new(env!("CARGO_BIN_EXE_blendoc")).args(&args).output().expect("command executes");

	assert!(output.status.success(), "command should succeed");
	serde_json::from_slice(&output.stdout).expect("stdout should be valid json")
}

fn fixture_path(name: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}
