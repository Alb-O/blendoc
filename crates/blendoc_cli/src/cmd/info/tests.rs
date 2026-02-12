use crate::cmd::test_support::{fixture_path, run_blendoc_json};

#[test]
fn info_json_includes_pointer_diagnostics() {
	let fixture = fixture_path("v5.1_character.blend");
	let fixture = fixture.to_string_lossy().into_owned();
	let json = run_blendoc_json(&["info", &fixture, "--json"]);

	assert_eq!(json["version"], 501);
	assert_eq!(json["pointer_storage"], "stable_ids");

	let diag = &json["pointer_diagnostics"];
	assert!(diag["indexed_entries"].as_u64().is_some_and(|item| item > 100));
	assert!(diag["overlapping_ranges"].as_u64().is_some_and(|item| item > 0));
	assert!(diag["min_old"].as_str().is_some_and(|item| item.starts_with("0x")));
	assert!(diag["max_old"].as_str().is_some_and(|item| item.starts_with("0x")));
	assert!(diag["max_end"].as_str().is_some_and(|item| item.starts_with("0x")));
	assert!(json["top_codes"].as_array().is_some_and(|items| !items.is_empty()));
}

#[test]
fn info_json_reports_supported_pointer_storage_value() {
	let fixture = fixture_path("character.blend");
	let fixture = fixture.to_string_lossy().into_owned();
	let json = run_blendoc_json(&["info", &fixture, "--json"]);

	let storage = json["pointer_storage"].as_str().expect("pointer storage label should be present");
	assert!(matches!(storage, "address_ranges" | "stable_ids"));
}
