use thiserror::Error;

pub type Result<T> = std::result::Result<T, BlendError>;

#[derive(Debug, Error)]
pub enum BlendError {
	#[error("io: {0}")]
	Io(#[from] std::io::Error),
	#[error("unsupported compression or not a .blend (magic={magic:?})")]
	UnknownMagic { magic: [u8; 4] },
	#[error("decompressed data does not start with BLENDER magic")]
	NotBlendAfterDecompress,
	#[error("unsupported endianness (expected little-endian 'v')")]
	BigEndianUnsupported,
	#[error("unsupported file format version {version} (expected 1)")]
	UnsupportedFormatVersion { version: u16 },
	#[error("unsupported blender version {version} (expected >= 500)")]
	UnsupportedBlendVersion { version: u16 },
	#[error("invalid header")]
	InvalidHeader,
	#[error("unexpected eof at offset {at}, need {need} bytes, remaining {rem}")]
	UnexpectedEof { at: usize, need: usize, rem: usize },
	#[error("negative block length {len}")]
	NegativeBlockLength { len: i64 },
	#[error("negative block count {nr}")]
	NegativeBlockCount { nr: i64 },
	#[error("block length {len} at offset {at} exceeds remaining {rem}")]
	BlockLenOutOfRange { at: usize, len: u64, rem: usize },
	#[error("decompressed output exceeded limit {limit} bytes")]
	DecompressedTooLarge { limit: usize },
	#[error("DNA1 block not found")]
	DnaNotFound,
	#[error("DNA tag mismatch at {at}: expected {expected:?}, got {got:?}")]
	DnaBadTag { expected: [u8; 4], got: [u8; 4], at: usize },
	#[error("DNA index out of range for {kind}: idx={idx}, max={max}")]
	DnaIndexOutOfRange { kind: &'static str, idx: u32, max: u32 },
	#[error("DNA struct not found: {name}")]
	DnaStructNotFound { name: String },
	#[error("DNA duplicate struct type index {type_idx}: first={first}, second={second}")]
	DnaDuplicateStructType { type_idx: u16, first: u32, second: u32 },
	#[error("block not found: {code:?}")]
	BlockNotFound { code: [u8; 4] },
	#[error("invalid block code: {code}")]
	InvalidBlockCode { code: String },
	#[error("decode depth exceeded (max={max_depth})")]
	DecodeDepthExceeded { max_depth: u32 },
	#[error("decode array too large: count={count}, max={max}")]
	DecodeArrayTooLarge { count: usize, max: usize },
	#[error("decode missing SDNA struct index {sdna_nr}")]
	DecodeMissingSdna { sdna_nr: u32 },
	#[error("decode payload too small: need={need}, have={have}")]
	DecodePayloadTooSmall { need: usize, have: usize },
	#[error("decode layout mismatch in {type_name}: leftover={leftover}")]
	DecodeLayoutMismatch { type_name: String, leftover: usize },
	#[error("chase unresolved pointer: 0x{ptr:016x}")]
	ChaseUnresolvedPtr { ptr: u64 },
	#[error("chase null pointer")]
	ChaseNullPtr,
	#[error("chase pointer out of bounds: 0x{ptr:016x}")]
	ChasePtrOutOfBounds { ptr: u64 },
	#[error("chase cycle detected at 0x{ptr:016x}")]
	ChaseCycle { ptr: u64 },
	#[error("chase hop limit exceeded: max={max_hops}")]
	ChaseHopLimitExceeded { max_hops: usize },
	#[error("chase expected pointer field {field} on {struct_name}")]
	ChaseExpectedPtr { struct_name: String, field: &'static str },
	#[error("chase missing field {field} on {struct_name}")]
	ChaseMissingField { struct_name: String, field: &'static str },
	#[error("chase type mismatch: expected {expected}, got {got}")]
	ChaseTypeMismatch { expected: &'static str, got: String },
	#[error("chase slice out of bounds: start={start}, size={size}, payload={payload}")]
	ChaseSliceOob { start: usize, size: usize, payload: usize },
	#[error("invalid field path: {path}")]
	InvalidFieldPath { path: String },
}
