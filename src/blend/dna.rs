use crate::blend::bytes::Cursor;
use crate::blend::{BlendError, Result};

#[derive(Debug)]
pub struct Dna {
	pub names: Vec<Box<str>>,
	pub types: Vec<Box<str>>,
	pub tlen: Vec<u16>,
	pub structs: Vec<DnaStruct>,
	pub struct_for_type: Vec<Option<u32>>,
}

#[derive(Debug)]
pub struct DnaStruct {
	pub type_idx: u16,
	pub fields: Vec<DnaField>,
}

#[derive(Debug, Clone, Copy)]
pub struct DnaField {
	pub type_idx: u16,
	pub name_idx: u16,
}

impl Dna {
	pub fn parse(payload: &[u8]) -> Result<Self> {
		let mut cursor = Cursor::new(payload);

		expect_tag(&mut cursor, *b"SDNA")?;
		expect_tag(&mut cursor, *b"NAME")?;

		let name_count = cursor.read_u32_le()? as usize;
		let mut names = Vec::with_capacity(name_count);
		for _ in 0..name_count {
			names.push(read_lossy_string(&mut cursor)?);
		}
		cursor.align4()?;

		expect_tag(&mut cursor, *b"TYPE")?;
		let type_count = cursor.read_u32_le()? as usize;
		let mut types = Vec::with_capacity(type_count);
		for _ in 0..type_count {
			types.push(read_lossy_string(&mut cursor)?);
		}
		cursor.align4()?;

		expect_tag(&mut cursor, *b"TLEN")?;
		let mut tlen = Vec::with_capacity(type_count);
		for _ in 0..type_count {
			tlen.push(cursor.read_u16_le()?);
		}
		cursor.align4()?;

		expect_tag(&mut cursor, *b"STRC")?;
		let struct_count = cursor.read_u32_le()? as usize;
		let mut structs = Vec::with_capacity(struct_count);

		for _ in 0..struct_count {
			let type_idx = cursor.read_u16_le()?;
			check_index("struct.type_idx", u32::from(type_idx), types.len())?;

			let field_count = cursor.read_u16_le()? as usize;
			let mut fields = Vec::with_capacity(field_count);
			for _ in 0..field_count {
				let field_type_idx = cursor.read_u16_le()?;
				let field_name_idx = cursor.read_u16_le()?;
				check_index("field.type_idx", u32::from(field_type_idx), types.len())?;
				check_index("field.name_idx", u32::from(field_name_idx), names.len())?;
				fields.push(DnaField {
					type_idx: field_type_idx,
					name_idx: field_name_idx,
				});
			}

			structs.push(DnaStruct { type_idx, fields });
		}

		let mut struct_for_type = vec![None; types.len()];
		for (idx, item) in structs.iter().enumerate() {
			let slot = &mut struct_for_type[item.type_idx as usize];
			if let Some(first) = *slot {
				return Err(BlendError::DnaDuplicateStructType {
					type_idx: item.type_idx,
					first,
					second: idx as u32,
				});
			}
			*slot = Some(idx as u32);
		}

		Ok(Self {
			names,
			types,
			tlen,
			structs,
			struct_for_type,
		})
	}

	pub fn struct_by_sdna(&self, sdna_nr: u32) -> Option<&DnaStruct> {
		self.structs.get(sdna_nr as usize)
	}

	pub fn struct_by_type_idx(&self, type_idx: u16) -> Option<&DnaStruct> {
		self.struct_for_type
			.get(type_idx as usize)
			.and_then(|index| index.and_then(|value| self.structs.get(value as usize)))
	}

	pub fn type_name(&self, type_idx: u16) -> &str {
		&self.types[type_idx as usize]
	}

	pub fn field_name(&self, name_idx: u16) -> &str {
		&self.names[name_idx as usize]
	}
}

fn expect_tag(cursor: &mut Cursor<'_>, expected: [u8; 4]) -> Result<()> {
	let at = cursor.pos();
	let got = cursor.read_code4()?;
	if got != expected {
		return Err(BlendError::DnaBadTag { expected, got, at });
	}
	Ok(())
}

fn read_lossy_string(cursor: &mut Cursor<'_>) -> Result<Box<str>> {
	let bytes = cursor.read_cstring_bytes()?;
	Ok(String::from_utf8_lossy(bytes).into_owned().into_boxed_str())
}

fn check_index(kind: &'static str, idx: u32, len: usize) -> Result<()> {
	if (idx as usize) >= len {
		return Err(BlendError::DnaIndexOutOfRange {
			kind,
			idx,
			max: len.saturating_sub(1) as u32,
		});
	}
	Ok(())
}
