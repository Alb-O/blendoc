use super::*;

fn map_with_entries(entries: &[(&str, &[(&str, &str)])]) -> FixtureMap {
	let mut map = FixtureMap::new();
	for (folder, files) in entries {
		let mut file_map = BTreeMap::new();
		for (path, url) in *files {
			file_map.insert((*path).to_owned(), (*url).to_owned());
		}
		map.insert((*folder).to_owned(), file_map);
	}
	map
}

#[test]
fn plan_includes_nested_paths() {
	let fixtures = map_with_entries(&[("shaderball", &[("textures/brown_photostudio_02_4k.exr", "https://example.com/file.exr")])]);

	let root = Path::new("fixtures/blendfiles");
	let plan = build_download_plan(&fixtures, root, None).expect("plan builds");

	assert_eq!(plan.len(), 1);
	assert_eq!(plan[0].destination, root.join("shaderball").join("textures/brown_photostudio_02_4k.exr"));
}

#[test]
fn plan_respects_folder_filter() {
	let fixtures = map_with_entries(&[
		("simple", &[("a.blend", "https://example.com/a")]),
		("sound", &[("main.blend", "https://example.com/main")]),
	]);

	let plan = build_download_plan(&fixtures, Path::new("fixtures/blendfiles"), Some("sound")).expect("filtered plan builds");

	assert_eq!(plan.len(), 1);
	assert_eq!(plan[0].folder, "sound");
	assert_eq!(plan[0].relative_path, "main.blend");
}

#[test]
fn plan_rejects_path_traversal() {
	let fixtures = map_with_entries(&[("bad", &[("../oops.blend", "https://example.com/oops")])]);

	let err = build_download_plan(&fixtures, Path::new("fixtures/blendfiles"), None).expect_err("path traversal should fail");

	let msg = err.to_string();
	assert!(msg.contains("path traversal"), "unexpected error message: {msg}");
}

#[test]
fn resolve_root_maps_blendfiles_to_fixture_tree() {
	let resolved = resolve_root(Path::new("blendfiles"));

	assert!(resolved.ends_with(Path::new("fixtures").join("blendfiles")));
}
