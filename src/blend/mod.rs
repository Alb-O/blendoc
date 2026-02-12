mod bhead;
mod block;
mod bytes;
mod chase;
mod chase_path;
mod compression;
mod decode;
mod dna;
mod error;
mod file;
mod header;
mod path;
mod pointer;
mod value;

/// Parsed block header record.
pub use bhead::BHead;
/// Block container and iterator types.
pub use block::{Block, BlockIter};
/// One-step pointer chase helpers.
pub use chase::{ChaseMeta, chase_ptr_to_struct, chase_scene_camera};
/// Generic path-based pointer chase API.
pub use chase_path::{ChasePolicy, ChaseResult, ChaseStop, ChaseStopReason, StopMode, chase_from_block_code, chase_from_ptr};
/// Compression detection result.
pub use compression::Compression;
/// SDNA-driven decoding entry points and options.
pub use decode::{DecodeOptions, decode_block_instances, decode_struct_instance};
/// SDNA schema representation.
pub use dna::{Dna, DnaField, DnaStruct};
/// Error and result aliases.
pub use error::{BlendError, Result};
/// File abstraction and block statistics.
pub use file::{BlendFile, BlockStats};
/// File header representation.
pub use header::BlendHeader;
/// Field path parser types.
pub use path::{FieldPath, PathStep};
/// Pointer index and resolution types.
pub use pointer::{PointerIndex, PtrEntry, ResolvedPtr, TypedResolvedPtr};
/// Decoded runtime value types.
pub use value::{FieldValue, StructValue, Value};
