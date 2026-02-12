#![allow(missing_docs)]

use std::path::Path;
use std::process::Command;

#[test]
fn show_expands_pointer_targets_when_enabled() {
	let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("character.blend");

	let output = Command::new(env!("CARGO_BIN_EXE_blendoc"))
		.arg("show")
		.arg(&fixture)
		.arg("--id")
		.arg("WOWorld")
		.arg("--expand-depth")
		.arg("1")
		.arg("--expand-max-nodes")
		.arg("8")
		.output()
		.expect("show command executes");

	assert!(output.status.success(), "show command should succeed");
	let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
	assert!(stdout.contains("nodetree = 0x"), "expected nodetree pointer line");
	assert!(stdout.contains("-> NTShader Nodetree"), "expected pointer annotation");
	assert!(stdout.contains("bNodeTree {"), "expected expanded nested struct output");
}
