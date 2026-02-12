use crate::cmd::test_support::{fixture_path, run_blendoc_json};

#[test]
fn show_json_output_is_valid_and_structured() {
	let fixture = fixture_path("character.blend");
	let fixture = fixture.to_string_lossy().into_owned();
	let json = run_blendoc_json(&["show", &fixture, "--id", "WOWorld", "--json"]);

	assert_eq!(json["root"], "id:WOWorld");
	assert!(json["canonical"].as_str().is_some_and(|item| item.starts_with("0x")));
	assert!(json["value"].is_object(), "expected top-level value object");
}
