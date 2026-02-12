use thiserror::Error;

/// Crate-local result type.
pub type Result<T> = std::result::Result<T, BlendError>;

/// Errors produced while reading, decoding, and traversing `.blend` data.
#[derive(Debug, Error)]
pub enum BlendError {
	/// Filesystem or stream IO failure.
	#[error("io: {0}")]
	Io(#[from] std::io::Error),
	/// Unknown leading file magic.
	#[error("unsupported compression or not a .blend (magic={magic:?})")]
	UnknownMagic {
		/// First up-to-4 bytes of the stream.
		magic: [u8; 4],
	},
	/// Decompressed stream did not start with `BLENDER`.
	#[error("decompressed data does not start with BLENDER magic")]
	NotBlendAfterDecompress,
	/// Endianness marker is not little-endian.
	#[error("unsupported endianness (expected little-endian 'v')")]
	BigEndianUnsupported,
	/// Unsupported container format version.
	#[error("unsupported file format version {version} (expected 1)")]
	UnsupportedFormatVersion {
		/// Parsed format version.
		version: u16,
	},
	/// Unsupported Blender major/minor version.
	#[error("unsupported blender version {version} (expected >= 500)")]
	UnsupportedBlendVersion {
		/// Parsed Blender version digits.
		version: u16,
	},
	/// Invalid or malformed file header.
	#[error("invalid header")]
	InvalidHeader,
	/// Not enough bytes remained for a requested read.
	#[error("unexpected eof at offset {at}, need {need} bytes, remaining {rem}")]
	UnexpectedEof {
		/// Byte offset where the read was attempted.
		at: usize,
		/// Requested bytes.
		need: usize,
		/// Bytes still available.
		rem: usize,
	},
	/// Block payload length was negative.
	#[error("negative block length {len}")]
	NegativeBlockLength {
		/// Parsed signed length.
		len: i64,
	},
	/// Block element count was negative.
	#[error("negative block count {nr}")]
	NegativeBlockCount {
		/// Parsed signed element count.
		nr: i64,
	},
	/// Block payload would exceed remaining file data.
	#[error("block length {len} at offset {at} exceeds remaining {rem}")]
	BlockLenOutOfRange {
		/// Block header file offset.
		at: usize,
		/// Declared payload length.
		len: u64,
		/// Remaining bytes in cursor.
		rem: usize,
	},
	/// Decompression output exceeded configured safety limit.
	#[error("decompressed output exceeded limit {limit} bytes")]
	DecompressedTooLarge {
		/// Maximum allowed output bytes.
		limit: usize,
	},
	/// No DNA1 block was found.
	#[error("DNA1 block not found")]
	DnaNotFound,
	/// Unexpected DNA section tag.
	#[error("DNA tag mismatch at {at}: expected {expected:?}, got {got:?}")]
	DnaBadTag {
		/// Expected section tag.
		expected: [u8; 4],
		/// Actual section tag.
		got: [u8; 4],
		/// Cursor offset of the tag read.
		at: usize,
	},
	/// Out-of-range index inside DNA tables.
	#[error("DNA index out of range for {kind}: idx={idx}, max={max}")]
	DnaIndexOutOfRange {
		/// Logical index kind being validated.
		kind: &'static str,
		/// Offending index value.
		idx: u32,
		/// Maximum valid index.
		max: u32,
	},
	/// Requested DNA struct name was not found.
	#[error("DNA struct not found: {name}")]
	DnaStructNotFound {
		/// Requested struct name.
		name: String,
	},
	/// Duplicate type->struct mapping in DNA `STRC` section.
	#[error("DNA duplicate struct type index {type_idx}: first={first}, second={second}")]
	DnaDuplicateStructType {
		/// Duplicate type index.
		type_idx: u16,
		/// First struct index observed.
		first: u32,
		/// Second struct index observed.
		second: u32,
	},
	/// Requested block code was not found.
	#[error("block not found: {code:?}")]
	BlockNotFound {
		/// Requested 4-byte block code.
		code: [u8; 4],
	},
	/// CLI block code argument was invalid.
	#[error("invalid block code: {code}")]
	InvalidBlockCode {
		/// User-provided code string.
		code: String,
	},
	/// Decoder recursion depth exceeded configured limit.
	#[error("decode depth exceeded (max={max_depth})")]
	DecodeDepthExceeded {
		/// Configured depth ceiling.
		max_depth: u32,
	},
	/// Requested decode array length exceeded configured limit.
	#[error("decode array too large: count={count}, max={max}")]
	DecodeArrayTooLarge {
		/// Requested array length.
		count: usize,
		/// Maximum permitted array length.
		max: usize,
	},
	/// SDNA struct id referenced by block or pointer is missing.
	#[error("decode missing SDNA struct index {sdna_nr}")]
	DecodeMissingSdna {
		/// Missing SDNA struct index.
		sdna_nr: u32,
	},
	/// Block payload was too short for requested decode size.
	#[error("decode payload too small: need={need}, have={have}")]
	DecodePayloadTooSmall {
		/// Required number of bytes.
		need: usize,
		/// Available bytes.
		have: usize,
	},
	/// Strict layout mode detected trailing undecoded bytes.
	#[error("decode layout mismatch in {type_name}: leftover={leftover}")]
	DecodeLayoutMismatch {
		/// Struct type name being decoded.
		type_name: String,
		/// Unconsumed bytes.
		leftover: usize,
	},
	/// Pointer resolver could not map non-zero pointer.
	#[error("chase unresolved pointer: 0x{ptr:016x}")]
	ChaseUnresolvedPtr {
		/// Pointer value that failed to resolve.
		ptr: u64,
	},
	/// Pointer traversal hit null where disallowed.
	#[error("chase null pointer")]
	ChaseNullPtr,
	/// Pointer resolved outside known element bounds.
	#[error("chase pointer out of bounds: 0x{ptr:016x}")]
	ChasePtrOutOfBounds {
		/// Pointer value that resolved to an invalid element region.
		ptr: u64,
	},
	/// Cycle detected while pointer chasing.
	#[error("chase cycle detected at 0x{ptr:016x}")]
	ChaseCycle {
		/// Canonical pointer participating in the cycle.
		ptr: u64,
	},
	/// Pointer chase exceeded configured hop budget.
	#[error("chase hop limit exceeded: max={max_hops}")]
	ChaseHopLimitExceeded {
		/// Maximum allowed dereference hops.
		max_hops: usize,
	},
	/// Struct field exists but is not a pointer value.
	#[error("chase expected pointer field {field} on {struct_name}")]
	ChaseExpectedPtr {
		/// Struct type name.
		struct_name: String,
		/// Field that was expected to be a pointer.
		field: &'static str,
	},
	/// Requested field is missing on decoded struct.
	#[error("chase missing field {field} on {struct_name}")]
	ChaseMissingField {
		/// Struct type name.
		struct_name: String,
		/// Missing field name.
		field: &'static str,
	},
	/// Runtime type mismatch while chasing or decoding.
	#[error("chase type mismatch: expected {expected}, got {got}")]
	ChaseTypeMismatch {
		/// Expected logical value kind.
		expected: &'static str,
		/// Actual logical value kind.
		got: String,
	},
	/// Requested element slice exceeded payload bounds.
	#[error("chase slice out of bounds: start={start}, size={size}, payload={payload}")]
	ChaseSliceOob {
		/// Requested start byte within payload.
		start: usize,
		/// Requested slice size.
		size: usize,
		/// Available payload length.
		payload: usize,
	},
	/// Path expression syntax is invalid.
	#[error("invalid field path: {path}")]
	InvalidFieldPath {
		/// Original user-provided path string.
		path: String,
	},
}
