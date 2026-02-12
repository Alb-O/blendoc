mod bhead;
mod block;
mod bytes;
mod chase;
mod chase_path;
mod compression;
mod decl;
mod decode;
mod dna;
mod error;
mod file;
mod graph;
mod header;
mod id;
mod idgraph;
mod path;
mod pointer;
mod refs;
mod route;
mod value;
mod walk;
mod xref;

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
pub use decode::{DecodeOptions, decode_block_instances, decode_ptr_instance, decode_struct_instance};
/// SDNA schema representation.
pub use dna::{Dna, DnaField, DnaStruct};
/// Error and result aliases.
pub use error::{BlendError, Result};
/// File abstraction and block statistics.
pub use file::{BlendFile, BlockStats};
/// Graph extraction types and entry points.
pub use graph::{GraphEdge, GraphNode, GraphOptions, GraphResult, GraphTruncation, build_graph_from_ptr};
/// File header representation.
pub use header::BlendHeader;
/// ID-root block scan output and helpers.
pub use id::{IdIndex, IdRecord, scan_id_blocks};
/// Whole-file ID graph extraction types and entry points.
pub use idgraph::{IdGraphEdge, IdGraphNode, IdGraphOptions, IdGraphResult, IdGraphTruncation, build_id_graph};
/// Field path parser types.
pub use path::{FieldPath, PathStep};
/// Pointer index and resolution types.
pub use pointer::{PointerIndex, PtrEntry, ResolvedPtr, TypedResolvedPtr};
/// Pointer-reference scan output and options.
pub use refs::{RefRecord, RefScanOptions, RefTarget, scan_refs_from_ptr};
/// Route-finding types and entry points.
pub use route::{RouteEdge, RouteOptions, RouteResult, RouteTruncation, find_route_between_ptrs};
/// Decoded runtime value types.
pub use value::{FieldValue, StructValue, Value};
/// Linked-list walk types and entry points.
pub use walk::{WalkItem, WalkOptions, WalkResult, WalkStop, WalkStopReason, walk_ptr_chain};
/// Inbound reference query types and entry points.
pub use xref::{InboundRef, XrefOptions, find_inbound_refs_to_ptr};
