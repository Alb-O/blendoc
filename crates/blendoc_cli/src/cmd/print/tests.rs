use blendoc::blend::{BHead, Block, Dna, DnaField, DnaStruct, IdIndex, IdRecord, PointerIndex, PtrEntry};

use super::{PrintCtx, PtrAnnotCtx, format_ptr};
use crate::cmd::test_support::{fixture_path, run_blendoc};

fn test_dna() -> Dna {
	Dna {
		endianness: blendoc::blend::Endianness::Little,
		pointer_size: 8,
		names: vec!["*next".into()],
		types: vec!["Node".into()],
		tlen: vec![8],
		structs: vec![DnaStruct {
			type_idx: 0,
			fields: vec![DnaField { type_idx: 0, name_idx: 0 }],
		}],
		struct_for_type: vec![Some(0)],
	}
}

fn make_index<'a>(payload: &'a [u8], start_old: u64, code: [u8; 4]) -> PointerIndex<'a> {
	let block = Block {
		head: BHead {
			code,
			sdna_nr: 0,
			old: start_old,
			len: payload.len() as u64,
			nr: 1,
		},
		payload,
		file_offset: 0,
	};
	PointerIndex::from_entries_for_test(vec![PtrEntry {
		start_old,
		end_old: start_old + payload.len() as u64,
		block,
	}])
}

#[test]
fn ptr_annotation_uses_id_label_when_available() {
	let payload = 0_u64.to_le_bytes();
	let index = make_index(&payload, 0x2000, *b"DATA");
	let dna = test_dna();
	let ids = IdIndex::build(vec![IdRecord {
		old_ptr: 0x2000,
		code: *b"ID\0\0",
		sdna_nr: 0,
		type_name: "Node".into(),
		id_name: "IDNode".into(),
		next: None,
		prev: None,
		lib: None,
	}]);

	let ctx = PrintCtx::new(
		Some(PtrAnnotCtx {
			dna: &dna,
			index: &index,
			ids: &ids,
		}),
		true,
		None,
		64,
	);

	let rendered = format_ptr(0x2000, Some(&ctx));
	assert!(rendered.contains("-> IDNode(Node)"));
}

#[test]
fn ptr_annotation_uses_type_for_non_id_target() {
	let payload = 0_u64.to_le_bytes();
	let index = make_index(&payload, 0x3000, *b"DATA");
	let dna = test_dna();
	let ids = IdIndex::build(Vec::new());

	let ctx = PrintCtx::new(
		Some(PtrAnnotCtx {
			dna: &dna,
			index: &index,
			ids: &ids,
		}),
		true,
		None,
		64,
	);

	let rendered = format_ptr(0x3000, Some(&ctx));
	assert!(rendered.contains("-> Node@0x0000000000003000"));
}

#[test]
fn ptr_annotation_marks_unresolved_targets() {
	let payload = 0_u64.to_le_bytes();
	let index = make_index(&payload, 0x4000, *b"DATA");
	let dna = test_dna();
	let ids = IdIndex::build(Vec::new());

	let ctx = PrintCtx::new(
		Some(PtrAnnotCtx {
			dna: &dna,
			index: &index,
			ids: &ids,
		}),
		true,
		None,
		64,
	);

	let rendered = format_ptr(0x9999, Some(&ctx));
	assert!(rendered.ends_with("(unresolved)"));
}

#[test]
fn show_prints_pointer_annotations() {
	let fixture = fixture_path("character.blend");
	let fixture = fixture.to_string_lossy().into_owned();
	let output = run_blendoc(&["show", &fixture, "--id", "WOWorld", "--annotate-ptrs"]);

	assert!(output.status.success(), "show command should succeed");
	let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
	assert!(stdout.contains("nodetree = 0x"), "expected nodetree pointer in output");
	assert!(stdout.contains("-> NTShader Nodetree"), "expected annotated nodetree pointer");
}

#[test]
fn show_expands_pointer_targets_when_enabled() {
	let fixture = fixture_path("character.blend");
	let fixture = fixture.to_string_lossy().into_owned();
	let output = run_blendoc(&["show", &fixture, "--id", "WOWorld", "--expand-depth", "1", "--expand-max-nodes", "8"]);

	assert!(output.status.success(), "show command should succeed");
	let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
	assert!(stdout.contains("nodetree = 0x"), "expected nodetree pointer line");
	assert!(stdout.contains("-> NTShader Nodetree"), "expected pointer annotation");
	assert!(stdout.contains("bNodeTree {"), "expected expanded nested struct output");
}
