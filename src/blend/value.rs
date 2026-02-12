/// Runtime value emitted by SDNA-driven decoding.
#[derive(Debug, Clone)]
pub enum Value {
	/// Explicit null marker.
	Null,
	/// Boolean scalar.
	Bool(bool),
	/// Signed integer scalar.
	I64(i64),
	/// Unsigned integer scalar.
	U64(u64),
	/// 32-bit float scalar.
	F32(f32),
	/// 64-bit float scalar.
	F64(f64),
	/// Opaque byte payload.
	Bytes(Vec<u8>),
	/// UTF-8 lossy decoded string.
	String(Box<str>),
	/// Raw pointer value from blend data.
	Ptr(u64),
	/// Homogeneous or heterogeneous sequence.
	Array(Vec<Value>),
	/// Struct-shaped decoded value.
	Struct(StructValue),
}

/// Decoded struct value with field names preserved.
#[derive(Debug, Clone)]
pub struct StructValue {
	/// Struct type name from DNA.
	pub type_name: Box<str>,
	/// Decoded field values in declaration order.
	pub fields: Vec<FieldValue>,
}

/// Named decoded field.
#[derive(Debug, Clone)]
pub struct FieldValue {
	/// Field identifier.
	pub name: Box<str>,
	/// Decoded field payload.
	pub value: Value,
}
