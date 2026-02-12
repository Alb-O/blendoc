use std::collections::HashSet;
use std::sync::Arc;

use crate::blend::{BlendError, Dna, IdIndex, PointerIndex, RefScanOptions, Result, StopMode, scan_refs_from_ptr};

/// Stop reason for linked-list walk traversal.
#[derive(Debug, Clone)]
pub enum WalkStopReason {
	/// Next pointer was null.
	NullNext,
	/// Next pointer was non-zero but unresolved.
	UnresolvedNext(u64),
	/// Canonical next node was already visited.
	Cycle(u64),
	/// Next field path was not found in scanned refs.
	MissingNextField {
		/// Requested next field path.
		field: Arc<str>,
	},
}

/// Stop metadata with traversal step index.
#[derive(Debug, Clone)]
pub struct WalkStop {
	/// Item index where stop occurred.
	pub step: usize,
	/// Structured stop reason.
	pub reason: WalkStopReason,
}

/// One visited item in a pointer walk.
#[derive(Debug, Clone)]
pub struct WalkItem {
	/// Zero-based visit index.
	pub index: usize,
	/// Canonical pointer for the visited node.
	pub canonical: u64,
	/// Block code containing this node.
	pub code: [u8; 4],
	/// SDNA struct index for this node.
	pub sdna_nr: u32,
	/// Resolved type name.
	pub type_name: Arc<str>,
	/// Optional ID name annotation.
	pub id_name: Option<Arc<str>>,
}

/// Linked-list traversal options.
#[derive(Debug, Clone)]
pub struct WalkOptions {
	/// Field path for the next pointer.
	pub next_field: Arc<str>,
	/// Maximum number of items to visit.
	pub max_steps: usize,
	/// Ref scan behavior used to discover `next_field`.
	pub ref_scan: RefScanOptions,
	/// Action when next pointer is null.
	pub on_null: StopMode,
	/// Action when next pointer is unresolved/missing.
	pub on_unresolved: StopMode,
	/// Action when cycle is detected.
	pub on_cycle: StopMode,
}

impl Default for WalkOptions {
	fn default() -> Self {
		Self {
			next_field: Arc::<str>::from("next"),
			max_steps: 256,
			ref_scan: RefScanOptions {
				max_depth: 1,
				max_array_elems: 4096,
			},
			on_null: StopMode::Stop,
			on_unresolved: StopMode::Stop,
			on_cycle: StopMode::Stop,
		}
	}
}

/// Result of linked-list walking from a canonical start.
#[derive(Debug, Clone)]
pub struct WalkResult {
	/// Visited items in order.
	pub items: Vec<WalkItem>,
	/// Optional stop reason when traversal ended before max steps.
	pub stop: Option<WalkStop>,
}

/// Walk a pointer chain by repeatedly following `next_field`.
pub fn walk_ptr_chain<'a>(dna: &Dna, index: &PointerIndex<'a>, ids: &IdIndex, start_ptr: u64, options: &WalkOptions) -> Result<WalkResult> {
	if start_ptr == 0 {
		return match options.on_null {
			StopMode::Stop => Ok(WalkResult {
				items: Vec::new(),
				stop: Some(WalkStop {
					step: 0,
					reason: WalkStopReason::NullNext,
				}),
			}),
			StopMode::Error => Err(BlendError::ChaseNullPtr),
		};
	}

	let mut items = Vec::new();
	let mut visited = HashSet::new();

	let mut current = match index.canonical_ptr(dna, start_ptr) {
		Some(value) => value,
		None => {
			return match options.on_unresolved {
				StopMode::Stop => Ok(WalkResult {
					items,
					stop: Some(WalkStop {
						step: 0,
						reason: WalkStopReason::UnresolvedNext(start_ptr),
					}),
				}),
				StopMode::Error => Err(BlendError::ChaseUnresolvedPtr { ptr: start_ptr }),
			};
		}
	};

	for step in 0..options.max_steps {
		let typed = index.resolve_typed(dna, current).ok_or(BlendError::ChaseUnresolvedPtr { ptr: current })?;
		if typed.element_index.is_none() {
			return Err(BlendError::ChasePtrOutOfBounds { ptr: current });
		}

		let type_name = dna
			.struct_by_sdna(typed.base.entry.block.head.sdna_nr)
			.map(|item| dna.type_name(item.type_idx))
			.unwrap_or("<unknown>");

		items.push(WalkItem {
			index: step,
			canonical: current,
			code: typed.base.entry.block.head.code,
			sdna_nr: typed.base.entry.block.head.sdna_nr,
			type_name: Arc::<str>::from(type_name),
			id_name: ids.get_by_ptr(current).map(|item| Arc::<str>::from(item.id_name.as_ref())),
		});

		visited.insert(current);

		let refs = scan_refs_from_ptr(dna, index, ids, current, &options.ref_scan)?;
		let Some(next_ref) = refs.iter().find(|item| item.field.as_ref() == options.next_field.as_ref()) else {
			let reason = WalkStopReason::MissingNextField {
				field: options.next_field.clone(),
			};
			return match options.on_unresolved {
				StopMode::Stop => Ok(WalkResult {
					items,
					stop: Some(WalkStop { step, reason }),
				}),
				StopMode::Error => Err(BlendError::WalkMissingNextField {
					field: options.next_field.to_string(),
				}),
			};
		};

		if next_ref.ptr == 0 {
			return match options.on_null {
				StopMode::Stop => Ok(WalkResult {
					items,
					stop: Some(WalkStop {
						step,
						reason: WalkStopReason::NullNext,
					}),
				}),
				StopMode::Error => Err(BlendError::ChaseNullPtr),
			};
		}

		let Some(target) = &next_ref.resolved else {
			return match options.on_unresolved {
				StopMode::Stop => Ok(WalkResult {
					items,
					stop: Some(WalkStop {
						step,
						reason: WalkStopReason::UnresolvedNext(next_ref.ptr),
					}),
				}),
				StopMode::Error => Err(BlendError::ChaseUnresolvedPtr { ptr: next_ref.ptr }),
			};
		};

		if visited.contains(&target.canonical) {
			return match options.on_cycle {
				StopMode::Stop => Ok(WalkResult {
					items,
					stop: Some(WalkStop {
						step,
						reason: WalkStopReason::Cycle(target.canonical),
					}),
				}),
				StopMode::Error => Err(BlendError::ChaseCycle { ptr: target.canonical }),
			};
		}

		current = target.canonical;
	}

	Ok(WalkResult { items, stop: None })
}
