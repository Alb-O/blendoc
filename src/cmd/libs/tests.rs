use crate::cmd::test_support::{fixture_path, run_blendoc_json};

#[test]
fn libs_json_reports_linked_sword_library() {
	let fixture = fixture_path("character.blend");
	let fixture = fixture.to_string_lossy().into_owned();
	let json = run_blendoc_json(&["libs", &fixture, "--json"]);

	assert!(json["libraries"].as_array().is_some_and(|items| !items.is_empty()));
	assert!(
		json["libraries"]
			.as_array()
			.expect("libraries should be array")
			.iter()
			.any(|item| item["library_path"].as_str().is_some_and(|path| path.contains("sword.blend")))
	);
}

#[test]
fn libs_json_reports_no_libraries_for_source_sword_file() {
	let fixture = fixture_path("sword.blend");
	let fixture = fixture.to_string_lossy().into_owned();
	let json = run_blendoc_json(&["libs", &fixture, "--json"]);

	assert!(json["libraries"].as_array().is_some_and(|items| items.is_empty()));
}

#[test]
fn linked_only_keeps_linked_object_in_character() {
	let fixture = fixture_path("character.blend");
	let fixture = fixture.to_string_lossy().into_owned();
	let json = run_blendoc_json(&["libs", &fixture, "--linked-only", "--json"]);

	assert!(
		json["ids"]
			.as_array()
			.expect("ids should be array")
			.iter()
			.any(|item| item["id_name"] == "OBsword_object" && item["linked"] == true)
	);
}

#[test]
fn ids_refs_and_show_json_include_link_metadata() {
	let fixture = fixture_path("character.blend");
	let fixture = fixture.to_string_lossy().into_owned();

	let ids_json = run_blendoc_json(&["ids", &fixture, "--json"]);
	let sword = ids_json
		.as_array()
		.expect("ids json should be array")
		.iter()
		.find(|item| item["id_name"] == "OBsword_object")
		.expect("sword object row exists");
	assert_eq!(sword["linked"], true);
	assert!(matches!(sword["link_confidence"].as_str(), Some("medium") | Some("high")));

	let refs_json = run_blendoc_json(&["refs", &fixture, "--id", "OBsword_object", "--json"]);
	assert_eq!(refs_json["owner_linked"], true);
	assert!(matches!(refs_json["owner_link_confidence"].as_str(), Some("medium") | Some("high")));

	let show_json = run_blendoc_json(&["show", &fixture, "--id", "OBsword_object", "--json"]);
	assert_eq!(show_json["root_linked"], true);
	assert!(matches!(show_json["root_link_confidence"].as_str(), Some("medium") | Some("high")));
}
