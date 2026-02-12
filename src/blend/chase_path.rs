use std::collections::{HashMap, HashSet};

use crate::blend::{
	BlendError, BlendFile, ChaseMeta, DecodeOptions, Dna, FieldPath, PathStep, PointerIndex, Result, StructValue, Value, decode_block_instances,
	decode_struct_instance,
};

/// Behavior when a traversal stop condition is encountered.
#[derive(Debug, Clone, Copy)]
pub enum StopMode {
	/// Return a `ChaseResult` with a stop reason.
	Stop,
	/// Return an immediate error.
	Error,
}

/// Policy controls for generic path-based pointer traversal.
#[derive(Debug, Clone)]
pub struct ChasePolicy {
	/// Maximum number of pointer dereference hops.
	pub max_hops: usize,
	/// Maximum number of canonical visited nodes.
	pub max_visited: usize,
	/// Default array index used when field access hits an array.
	pub array_default_index: Option<usize>,
	/// Action when pointer is null.
	pub on_null_ptr: StopMode,
	/// Action when pointer cannot be resolved.
	pub on_unresolved_ptr: StopMode,
	/// Action when cycle or visited-limit condition is encountered.
	pub on_cycle: StopMode,
}

impl Default for ChasePolicy {
	fn default() -> Self {
		Self {
			max_hops: 64,
			max_visited: 10_000,
			array_default_index: Some(0),
			on_null_ptr: StopMode::Stop,
			on_unresolved_ptr: StopMode::Stop,
			on_cycle: StopMode::Error,
		}
	}
}

/// Reason traversal stopped before consuming all requested semantics.
#[derive(Debug, Clone)]
pub enum ChaseStopReason {
	/// Encountered a null pointer.
	NullPtr,
	/// Encountered a non-zero pointer that was not resolvable.
	UnresolvedPtr(u64),
	/// Encountered a previously visited canonical pointer.
	Cycle(u64),
	/// Requested struct field is absent.
	MissingField {
		/// Struct type where lookup failed.
		struct_name: String,
		/// Missing field name.
		field: String,
	},
	/// Operation required a struct value but found another kind.
	ExpectedStruct {
		/// Actual value kind.
		got: String,
	},
	/// Operation required an array value but found another kind.
	ExpectedArray {
		/// Actual value kind.
		got: String,
	},
	/// Array index was out of bounds.
	IndexOob {
		/// Requested index.
		index: usize,
		/// Array length.
		len: usize,
	},
}

/// Stop metadata with source step index.
#[derive(Debug, Clone)]
pub struct ChaseStop {
	/// Path step index that produced the stop.
	pub step_index: usize,
	/// Structured stop reason.
	pub reason: ChaseStopReason,
}

/// Result of path traversal and pointer chasing.
#[derive(Debug, Clone)]
pub struct ChaseResult {
	/// Final value reached by traversal.
	pub value: Value,
	/// Ordered metadata for performed pointer dereferences.
	pub hops: Vec<ChaseMeta>,
	/// Optional stop details when traversal ended early.
	pub stop: Option<ChaseStop>,
}

/// Start from the first block code match and chase a parsed field path.
pub fn chase_from_block_code<'a>(
	file: &'a BlendFile,
	dna: &Dna,
	index: &PointerIndex<'a>,
	root_code: [u8; 4],
	path: &FieldPath,
	decode: &DecodeOptions,
	policy: &ChasePolicy,
) -> Result<ChaseResult> {
	let block = file.find_first_block_by_code(root_code)?.ok_or(BlendError::BlockNotFound { code: root_code })?;
	let root = decode_block_instances(dna, &block, decode)?;
	chase_value(root, dna, index, path, decode, policy)
}

/// Start from a raw pointer and chase a parsed field path.
pub fn chase_from_ptr<'a>(
	dna: &Dna,
	index: &PointerIndex<'a>,
	root_ptr: u64,
	path: &FieldPath,
	decode: &DecodeOptions,
	policy: &ChasePolicy,
) -> Result<ChaseResult> {
	chase_value(Value::Ptr(root_ptr), dna, index, path, decode, policy)
}

fn chase_value<'a>(
	mut current: Value,
	dna: &Dna,
	index: &PointerIndex<'a>,
	path: &FieldPath,
	decode: &DecodeOptions,
	policy: &ChasePolicy,
) -> Result<ChaseResult> {
	let config = DerefConfig { decode, policy };
	let mut hops = Vec::new();
	let mut visited = HashSet::new();
	let mut decoded_cache: HashMap<u64, StructValue> = HashMap::new();

	for (step_index, step) in path.steps.iter().enumerate() {
		loop {
			match (step, current.clone()) {
				(PathStep::Field(field_name), Value::Struct(item)) => {
					let Some(field) = item.fields.iter().find(|candidate| candidate.name.as_ref() == field_name) else {
						return Ok(ChaseResult {
							value: current,
							hops,
							stop: Some(ChaseStop {
								step_index,
								reason: ChaseStopReason::MissingField {
									struct_name: item.type_name.to_string(),
									field: field_name.clone(),
								},
							}),
						});
					};
					current = field.value.clone();
					break;
				}
				(PathStep::Field(_), Value::Array(items)) => {
					let Some(default_index) = policy.array_default_index else {
						return Ok(ChaseResult {
							value: current,
							hops,
							stop: Some(ChaseStop {
								step_index,
								reason: ChaseStopReason::ExpectedStruct { got: "Array".to_owned() },
							}),
						});
					};

					if default_index >= items.len() {
						return Ok(ChaseResult {
							value: current,
							hops,
							stop: Some(ChaseStop {
								step_index,
								reason: ChaseStopReason::IndexOob {
									index: default_index,
									len: items.len(),
								},
							}),
						});
					}

					current = items[default_index].clone();
					continue;
				}
				(PathStep::Field(_), Value::Ptr(ptr)) => match deref_pointer(dna, index, ptr, &config, &mut hops, &mut visited, &mut decoded_cache)? {
					DerefOutcome::Struct(item) => {
						current = Value::Struct(item);
						continue;
					}
					DerefOutcome::Stop(reason) => {
						return Ok(ChaseResult {
							value: current,
							hops,
							stop: Some(ChaseStop { step_index, reason }),
						});
					}
				},
				(PathStep::Field(_), other) => {
					return Ok(ChaseResult {
						value: current,
						hops,
						stop: Some(ChaseStop {
							step_index,
							reason: ChaseStopReason::ExpectedStruct {
								got: value_kind(&other).to_owned(),
							},
						}),
					});
				}

				(PathStep::Index(index_value), Value::Array(items)) => {
					if *index_value >= items.len() {
						return Ok(ChaseResult {
							value: current,
							hops,
							stop: Some(ChaseStop {
								step_index,
								reason: ChaseStopReason::IndexOob {
									index: *index_value,
									len: items.len(),
								},
							}),
						});
					}

					current = items[*index_value].clone();
					break;
				}
				(PathStep::Index(_), Value::Ptr(ptr)) => match deref_pointer(dna, index, ptr, &config, &mut hops, &mut visited, &mut decoded_cache)? {
					DerefOutcome::Struct(item) => {
						current = Value::Struct(item);
						continue;
					}
					DerefOutcome::Stop(reason) => {
						return Ok(ChaseResult {
							value: current,
							hops,
							stop: Some(ChaseStop { step_index, reason }),
						});
					}
				},
				(PathStep::Index(_), other) => {
					return Ok(ChaseResult {
						value: current,
						hops,
						stop: Some(ChaseStop {
							step_index,
							reason: ChaseStopReason::ExpectedArray {
								got: value_kind(&other).to_owned(),
							},
						}),
					});
				}
			}
		}
	}

	let final_step = path.steps.len();
	loop {
		let Value::Ptr(ptr) = current.clone() else {
			break;
		};

		match deref_pointer(dna, index, ptr, &config, &mut hops, &mut visited, &mut decoded_cache)? {
			DerefOutcome::Struct(item) => {
				current = Value::Struct(item);
			}
			DerefOutcome::Stop(reason) => {
				return Ok(ChaseResult {
					value: current,
					hops,
					stop: Some(ChaseStop {
						step_index: final_step,
						reason,
					}),
				});
			}
		}
	}

	Ok(ChaseResult {
		value: current,
		hops,
		stop: None,
	})
}

enum DerefOutcome {
	Struct(StructValue),
	Stop(ChaseStopReason),
}

struct DerefConfig<'a> {
	decode: &'a DecodeOptions,
	policy: &'a ChasePolicy,
}

fn deref_pointer<'a>(
	dna: &Dna,
	index: &PointerIndex<'a>,
	ptr: u64,
	config: &DerefConfig<'_>,
	hops: &mut Vec<ChaseMeta>,
	visited: &mut HashSet<u64>,
	decoded_cache: &mut HashMap<u64, StructValue>,
) -> Result<DerefOutcome> {
	if hops.len() >= config.policy.max_hops {
		return Err(BlendError::ChaseHopLimitExceeded {
			max_hops: config.policy.max_hops,
		});
	}

	if ptr == 0 {
		return match config.policy.on_null_ptr {
			StopMode::Stop => Ok(DerefOutcome::Stop(ChaseStopReason::NullPtr)),
			StopMode::Error => Err(BlendError::ChaseNullPtr),
		};
	}

	let Some(typed) = index.resolve_typed(dna, ptr) else {
		return match config.policy.on_unresolved_ptr {
			StopMode::Stop => Ok(DerefOutcome::Stop(ChaseStopReason::UnresolvedPtr(ptr))),
			StopMode::Error => Err(BlendError::ChaseUnresolvedPtr { ptr }),
		};
	};

	let Some(element_index) = typed.element_index else {
		return match config.policy.on_unresolved_ptr {
			StopMode::Stop => Ok(DerefOutcome::Stop(ChaseStopReason::UnresolvedPtr(ptr))),
			StopMode::Error => Err(BlendError::ChasePtrOutOfBounds { ptr }),
		};
	};

	if visited.len() >= config.policy.max_visited {
		return match config.policy.on_cycle {
			StopMode::Stop => Ok(DerefOutcome::Stop(ChaseStopReason::Cycle(ptr))),
			StopMode::Error => Err(BlendError::ChaseCycle { ptr }),
		};
	}

	let Some(offset_bytes) = element_index.checked_mul(typed.struct_size) else {
		return Err(BlendError::ChaseSliceOob {
			start: usize::MAX,
			size: typed.struct_size,
			payload: typed.base.payload().len(),
		});
	};
	let canonical = typed.base.entry.start_old + offset_bytes as u64;

	if visited.contains(&canonical) {
		return match config.policy.on_cycle {
			StopMode::Stop => Ok(DerefOutcome::Stop(ChaseStopReason::Cycle(canonical))),
			StopMode::Error => Err(BlendError::ChaseCycle { ptr: canonical }),
		};
	}
	visited.insert(canonical);

	if let Some(cached) = decoded_cache.get(&canonical) {
		hops.push(ChaseMeta {
			ptr,
			resolved_block_code: typed.base.entry.block.head.code,
			sdna_nr: typed.base.entry.block.head.sdna_nr,
			element_index,
			element_offset: typed.element_offset,
			struct_size: typed.struct_size,
			block_old: typed.base.entry.start_old,
		});
		return Ok(DerefOutcome::Struct(cached.clone()));
	}

	let start = offset_bytes;
	let end = start.checked_add(typed.struct_size).ok_or(BlendError::ChaseSliceOob {
		start,
		size: typed.struct_size,
		payload: typed.base.payload().len(),
	})?;
	let bytes = typed.base.payload().get(start..end).ok_or(BlendError::ChaseSliceOob {
		start,
		size: typed.struct_size,
		payload: typed.base.payload().len(),
	})?;

	let value = decode_struct_instance(dna, typed.base.entry.block.head.sdna_nr, bytes, config.decode)?;
	decoded_cache.insert(canonical, value.clone());
	hops.push(ChaseMeta {
		ptr,
		resolved_block_code: typed.base.entry.block.head.code,
		sdna_nr: typed.base.entry.block.head.sdna_nr,
		element_index,
		element_offset: typed.element_offset,
		struct_size: typed.struct_size,
		block_old: typed.base.entry.start_old,
	});

	Ok(DerefOutcome::Struct(value))
}

fn value_kind(value: &Value) -> &'static str {
	match value {
		Value::Null => "Null",
		Value::Bool(_) => "Bool",
		Value::I64(_) => "I64",
		Value::U64(_) => "U64",
		Value::F32(_) => "F32",
		Value::F64(_) => "F64",
		Value::Bytes(_) => "Bytes",
		Value::String(_) => "String",
		Value::Ptr(_) => "Ptr",
		Value::Array(_) => "Array",
		Value::Struct(_) => "Struct",
	}
}
