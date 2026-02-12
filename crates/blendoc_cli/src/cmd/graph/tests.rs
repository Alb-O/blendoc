use crate::cmd::test_support::{fixture_path, run_blendoc_json};

#[test]
fn graph_json_output_contains_nodes_and_edges() {
	let fixture = fixture_path("character.blend");
	let fixture = fixture.to_string_lossy().into_owned();
	let json = run_blendoc_json(&["graph", &fixture, "--id", "SCScene", "--depth", "1", "--refs-depth", "1", "--json"]);

	assert_eq!(json["root"], "id:SCScene");
	assert!(json["nodes"].as_array().is_some_and(|items| !items.is_empty()), "expected graph nodes");
	assert!(json["edges"].as_array().is_some(), "expected graph edges array");
}
