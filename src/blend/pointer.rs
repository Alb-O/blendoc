use crate::blend::{BlendFile, Block, Dna, Result};

/// Range index for resolving old-memory pointers to blocks.
#[derive(Debug)]
pub struct PointerIndex<'a> {
	starts: Vec<u64>,
	entries: Vec<PtrEntry<'a>>,
}

/// One indexed pointer range entry.
#[derive(Debug, Clone, Copy)]
pub struct PtrEntry<'a> {
	/// Base old pointer address for this block payload.
	pub start_old: u64,
	/// Exclusive end address for this block payload range.
	pub end_old: u64,
	/// Source block metadata and payload.
	pub block: Block<'a>,
}

/// Result of mapping a pointer to an indexed range.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedPtr<'a> {
	/// Matched range entry.
	pub entry: PtrEntry<'a>,
	/// Byte offset of pointer inside matched entry.
	pub byte_offset: usize,
}

/// Resolved pointer annotated with SDNA element positioning.
#[derive(Debug, Clone, Copy)]
pub struct TypedResolvedPtr<'a> {
	/// Base untyped resolution.
	pub base: ResolvedPtr<'a>,
	/// Size in bytes of one decoded element.
	pub struct_size: usize,
	/// Element index when pointer lands within `nr * struct_size`.
	pub element_index: Option<usize>,
	/// Byte offset within the resolved element.
	pub element_offset: usize,
}

impl<'a> PointerIndex<'a> {
	/// Build a sorted index from caller-provided entries.
	///
	/// This is primarily useful for deterministic unit tests.
	pub fn from_entries_for_test(mut entries: Vec<PtrEntry<'a>>) -> Self {
		entries.sort_by_key(|entry| entry.start_old);
		let starts = entries.iter().map(|entry| entry.start_old).collect();
		Self { starts, entries }
	}

	/// Scan a file and build pointer ranges for non-empty blocks.
	pub fn build(file: &'a BlendFile) -> Result<Self> {
		let mut entries = Vec::new();

		for block in file.blocks() {
			let block = block?;
			if block.head.old == 0 || block.payload.is_empty() {
				continue;
			}

			let start_old = block.head.old;
			let end_old = start_old.saturating_add(block.payload.len() as u64);
			entries.push(PtrEntry { start_old, end_old, block });
		}

		Ok(Self::from_entries_for_test(entries))
	}

	/// Resolve a pointer to the containing payload range.
	pub fn resolve(&self, ptr: u64) -> Option<ResolvedPtr<'a>> {
		if ptr == 0 {
			return None;
		}

		let idx = self.starts.partition_point(|start| *start <= ptr);
		if idx == 0 {
			return None;
		}

		let entry = self.entries[idx - 1];
		if ptr >= entry.end_old {
			return None;
		}

		Some(ResolvedPtr {
			entry,
			byte_offset: (ptr - entry.start_old) as usize,
		})
	}

	/// Resolve a pointer and compute SDNA element position data.
	pub fn resolve_typed(&self, dna: &Dna, ptr: u64) -> Option<TypedResolvedPtr<'a>> {
		let base = self.resolve(ptr)?;
		let item = dna.struct_by_sdna(base.entry.block.head.sdna_nr)?;
		let struct_size = usize::from(dna.tlen[item.type_idx as usize]);

		if struct_size == 0 {
			return Some(TypedResolvedPtr {
				base,
				struct_size,
				element_index: None,
				element_offset: base.byte_offset,
			});
		}

		let nr = usize::try_from(base.entry.block.head.nr).ok()?;
		let max_bytes = struct_size.checked_mul(nr)?;
		let (element_index, element_offset) = if base.byte_offset < max_bytes {
			(Some(base.byte_offset / struct_size), base.byte_offset % struct_size)
		} else {
			(None, base.byte_offset % struct_size)
		};

		Some(TypedResolvedPtr {
			base,
			struct_size,
			element_index,
			element_offset,
		})
	}

	/// Canonicalize a pointer to the start of its resolved struct element.
	pub fn canonical_ptr(&self, dna: &Dna, ptr: u64) -> Option<u64> {
		let typed = self.resolve_typed(dna, ptr)?;
		let element_index = typed.element_index?;
		let offset = element_index.checked_mul(typed.struct_size)?;
		let offset = u64::try_from(offset).ok()?;
		typed.base.entry.start_old.checked_add(offset)
	}

	/// Return all indexed entries in sorted order.
	pub fn entries(&self) -> &[PtrEntry<'a>] {
		&self.entries
	}

	/// Return number of indexed entries.
	pub fn len(&self) -> usize {
		self.entries.len()
	}

	/// Return whether there are no indexed entries.
	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}
}

impl<'a> ResolvedPtr<'a> {
	/// Return full payload bytes for the matched block.
	pub fn payload(&self) -> &'a [u8] {
		self.entry.block.payload
	}

	/// Return a bounded slice starting at `byte_offset`.
	pub fn slice_from(&self, len: usize) -> Option<&'a [u8]> {
		let start = self.byte_offset;
		let end = start.checked_add(len)?;
		self.payload().get(start..end)
	}
}
