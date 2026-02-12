use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use blendoc::blend::{DecodeOptions, Dna, IdIndex, PointerIndex, StructValue, Value, decode_ptr_instance};

use crate::cmd::util::render_code;

/// Output truncation and formatting limits for decoded values.
#[derive(Debug, Clone, Copy)]
pub struct PrintOptions {
	/// Maximum number of fields printed for a single struct.
	pub max_fields_per_struct: usize,
	/// Maximum number of Unicode scalar values printed for strings.
	pub max_string_len: usize,
	/// Maximum number of elements printed for arrays.
	pub max_array_items: usize,
	/// Maximum recursive print depth for nested arrays/structs.
	pub max_print_depth: u32,
}

impl Default for PrintOptions {
	fn default() -> Self {
		Self {
			max_fields_per_struct: 80,
			max_string_len: 200,
			max_array_items: 16,
			max_print_depth: 6,
		}
	}
}

impl PrintOptions {
	/// Preset tuned for scene-sized output.
	pub fn for_scene_inspect() -> Self {
		Self {
			max_fields_per_struct: 40,
			max_string_len: 160,
			max_array_items: 8,
			max_print_depth: 4,
		}
	}
}

/// Context needed to annotate pointer values while printing.
pub struct PtrAnnotCtx<'a> {
	/// SDNA schema reference.
	pub dna: &'a Dna,
	/// Pointer index for resolution.
	pub index: &'a PointerIndex<'a>,
	/// ID lookup index for friendly labels.
	pub ids: &'a IdIndex,
}

/// Optional rendering context for pointer annotation.
pub struct PrintCtx<'a> {
	/// Optional pointer annotation dependencies.
	pub ptr_annot: Option<PtrAnnotCtx<'a>>,
	/// Whether pointer annotation should be applied.
	pub annotate_ptrs: bool,
	/// Optional decode options used for pointer expansion.
	pub decode: Option<&'a DecodeOptions>,
	/// Maximum expanded pointer nodes per print call.
	pub expand_max_nodes: usize,
	cache: RefCell<HashMap<u64, String>>,
	expand_stack: RefCell<Vec<u64>>,
	expand_count: Cell<usize>,
	decoded_cache: RefCell<HashMap<u64, StructValue>>,
}

impl<'a> PrintCtx<'a> {
	/// Create a print context.
	pub fn new(ptr_annot: Option<PtrAnnotCtx<'a>>, annotate_ptrs: bool, decode: Option<&'a DecodeOptions>, expand_max_nodes: usize) -> Self {
		Self {
			ptr_annot,
			annotate_ptrs,
			decode,
			expand_max_nodes,
			cache: RefCell::new(HashMap::new()),
			expand_stack: RefCell::new(Vec::new()),
			expand_count: Cell::new(0),
			decoded_cache: RefCell::new(HashMap::new()),
		}
	}
}

/// Print one decoded runtime value tree.
pub fn print_value(value: &Value, indent: usize, depth: u32, options: PrintOptions, ctx: Option<&PrintCtx<'_>>, expand_left: u32) {
	let pad = " ".repeat(indent);
	match value {
		Value::Null => println!("{}null", pad),
		Value::Bool(v) => println!("{}{v}", pad),
		Value::I64(v) => println!("{}{v}", pad),
		Value::U64(v) => println!("{}{v}", pad),
		Value::F32(v) => println!("{}{v}", pad),
		Value::F64(v) => println!("{}{v}", pad),
		Value::Bytes(v) => println!("{}bytes[{}]", pad, v.len()),
		Value::String(v) => println!("{}\"{}\"", pad, truncate(v, options.max_string_len)),
		Value::Ptr(v) => {
			println!("{}{}", pad, format_ptr(*v, ctx));
			print_ptr_expansion(*v, indent, depth, options, ctx, expand_left);
		}
		Value::Array(items) => {
			if depth >= options.max_print_depth {
				println!("{}[... {} items]", pad, items.len());
				return;
			}
			println!("{}[", pad);
			for item in items.iter().take(options.max_array_items) {
				print_value(item, indent + 2, depth + 1, options, ctx, expand_left);
			}
			if items.len() > options.max_array_items {
				println!("{}  ... {} more", pad, items.len() - options.max_array_items);
			}
			println!("{}]", pad);
		}
		Value::Struct(item) => {
			if depth >= options.max_print_depth {
				println!("{}{} {{ ... }}", pad, item.type_name);
				return;
			}
			println!("{}{} {{", pad, item.type_name);
			for field in item.fields.iter().take(options.max_fields_per_struct) {
				print!("{}  {} = ", pad, field.name);
				if matches!(field.value, Value::Struct(_) | Value::Array(_)) {
					println!();
					print_value(&field.value, indent + 4, depth + 1, options, ctx, expand_left);
				} else {
					print_value(&field.value, 0, depth + 1, options, ctx, expand_left);
				}
			}
			if item.fields.len() > options.max_fields_per_struct {
				println!("{}  ... {} more fields", pad, item.fields.len() - options.max_fields_per_struct);
			}
			println!("{}}}", pad);
		}
	}
}

fn print_ptr_expansion(ptr: u64, indent: usize, depth: u32, options: PrintOptions, ctx: Option<&PrintCtx<'_>>, expand_left: u32) {
	if ptr == 0 || expand_left == 0 {
		return;
	}

	let Some(ctx) = ctx else {
		return;
	};
	let Some(annot) = &ctx.ptr_annot else {
		return;
	};
	let Some(decode) = ctx.decode else {
		return;
	};

	let Some(canonical) = annot.index.canonical_ptr(annot.dna, ptr) else {
		return;
	};

	if ctx.expand_stack.borrow().contains(&canonical) {
		println!("{}... (cycle)", " ".repeat(indent + 2));
		return;
	}
	if ctx.expand_count.get() >= ctx.expand_max_nodes {
		println!("{}... (budget)", " ".repeat(indent + 2));
		return;
	}

	let decoded = if let Some(cached) = ctx.decoded_cache.borrow().get(&canonical) {
		cached.clone()
	} else {
		let Ok((resolved_canonical, struct_value)) = decode_ptr_instance(annot.dna, annot.index, ptr, decode) else {
			println!("{}... (unresolved)", " ".repeat(indent + 2));
			return;
		};
		ctx.decoded_cache.borrow_mut().insert(resolved_canonical, struct_value.clone());
		struct_value
	};

	ctx.expand_stack.borrow_mut().push(canonical);
	ctx.expand_count.set(ctx.expand_count.get() + 1);
	print_value(&Value::Struct(decoded), indent + 2, depth + 1, options, Some(ctx), expand_left - 1);
	ctx.expand_stack.borrow_mut().pop();
}

fn format_ptr(ptr: u64, ctx: Option<&PrintCtx<'_>>) -> String {
	let raw = format!("0x{ptr:016x}");
	if ptr == 0 {
		return raw;
	}

	let Some(ctx) = ctx else {
		return raw;
	};
	if !ctx.annotate_ptrs {
		return raw;
	}
	let Some(annot) = &ctx.ptr_annot else {
		return raw;
	};

	if let Some(cached) = ctx.cache.borrow().get(&ptr) {
		return cached.clone();
	}

	let rendered = match annot.index.resolve_typed(annot.dna, ptr) {
		Some(typed) if typed.element_index.is_some() => {
			if let Some(canonical) = annot.index.canonical_ptr(annot.dna, ptr) {
				let type_name = annot
					.dna
					.struct_by_sdna(typed.base.entry.block.head.sdna_nr)
					.map(|item| annot.dna.type_name(item.type_idx))
					.unwrap_or("<unknown>");

				if let Some(id) = annot.ids.get_by_ptr(canonical) {
					format!("{raw} -> {}({type_name})", id.id_name)
				} else {
					format!(
						"{raw} -> {type_name}@0x{canonical:016x} (code={})",
						render_code(typed.base.entry.block.head.code)
					)
				}
			} else {
				format!("{raw} (unresolved)")
			}
		}
		_ => format!("{raw} (unresolved)"),
	};

	ctx.cache.borrow_mut().insert(ptr, rendered.clone());
	rendered
}

fn truncate(input: &str, max_len: usize) -> String {
	if input.chars().count() <= max_len {
		return input.to_owned();
	}
	let out: String = input.chars().take(max_len).collect();
	format!("{out}...")
}

#[cfg(test)]
mod tests {
	use blendoc::blend::{BHead, Block, Dna, DnaField, DnaStruct, IdIndex, IdRecord, PointerIndex, PtrEntry};

	use super::{PrintCtx, PtrAnnotCtx, format_ptr};

	fn test_dna() -> Dna {
		Dna {
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
}
