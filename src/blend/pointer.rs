use crate::blend::{BlendFile, Block, Dna, Result};

#[derive(Debug)]
pub struct PointerIndex<'a> {
	starts: Vec<u64>,
	entries: Vec<PtrEntry<'a>>,
}

#[derive(Debug, Clone, Copy)]
pub struct PtrEntry<'a> {
	pub start_old: u64,
	pub end_old: u64,
	pub block: Block<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct ResolvedPtr<'a> {
	pub entry: PtrEntry<'a>,
	pub byte_offset: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct TypedResolvedPtr<'a> {
	pub base: ResolvedPtr<'a>,
	pub struct_size: usize,
	pub element_index: Option<usize>,
	pub element_offset: usize,
}

impl<'a> PointerIndex<'a> {
	#[doc(hidden)]
	pub fn from_entries_for_test(mut entries: Vec<PtrEntry<'a>>) -> Self {
		entries.sort_by_key(|entry| entry.start_old);
		let starts = entries.iter().map(|entry| entry.start_old).collect();
		Self { starts, entries }
	}

	pub fn build(file: &'a BlendFile) -> Result<Self> {
		let mut entries = Vec::new();

		for block in file.blocks() {
			let block = block?;
			if block.head.old == 0 || block.payload.is_empty() {
				continue;
			}

			let start_old = block.head.old;
			let end_old = start_old.checked_add(block.payload.len() as u64).unwrap_or(u64::MAX);
			entries.push(PtrEntry { start_old, end_old, block });
		}

		Ok(Self::from_entries_for_test(entries))
	}

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

	pub fn entries(&self) -> &[PtrEntry<'a>] {
		&self.entries
	}

	pub fn len(&self) -> usize {
		self.entries.len()
	}
}

impl<'a> ResolvedPtr<'a> {
	pub fn payload(&self) -> &'a [u8] {
		self.entry.block.payload
	}

	pub fn slice_from(&self, len: usize) -> Option<&'a [u8]> {
		let start = self.byte_offset;
		let end = start.checked_add(len)?;
		self.payload().get(start..end)
	}
}
