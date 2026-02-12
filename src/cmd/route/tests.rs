use crate::cmd::test_support::{fixture_path, run_blendoc_json};

#[test]
fn route_json_output_includes_path_edges_array() {
	let fixture = fixture_path("character.blend");
	let fixture = fixture.to_string_lossy().into_owned();
	let json = run_blendoc_json(&["route", &fixture, "--from-id", "SCScene", "--to-id", "WOWorld", "--json"]);

	assert_eq!(json["from"]["selector"], "id:SCScene");
	assert_eq!(json["to"]["selector"], "id:WOWorld");
	assert!(json["path_edges"].is_array(), "expected path_edges array");
}
