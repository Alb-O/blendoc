#[derive(Debug, Clone)]
pub enum Value {
	Null,
	Bool(bool),
	I64(i64),
	U64(u64),
	F32(f32),
	F64(f64),
	Bytes(Vec<u8>),
	String(Box<str>),
	Ptr(u64),
	Array(Vec<Value>),
	Struct(StructValue),
}

#[derive(Debug, Clone)]
pub struct StructValue {
	pub type_name: Box<str>,
	pub fields: Vec<FieldValue>,
}

#[derive(Debug, Clone)]
pub struct FieldValue {
	pub name: Box<str>,
	pub value: Value,
}
