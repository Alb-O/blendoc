use std::path::PathBuf;

use blendoc::blend::{BlendError, BlendFile, DnaStruct, Result};

pub fn run(path: PathBuf, struct_name: Option<String>) -> Result<()> {
	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;

	println!("path: {}", path.display());
	println!("compression: {}", blend.compression.as_str());
	println!("version: {}", blend.header.version);
	println!("dna_names: {}", dna.names.len());
	println!("dna_types: {}", dna.types.len());
	println!("dna_structs: {}", dna.structs.len());

	if let Some(name) = struct_name {
		let (sdna_idx, item) = find_struct_by_name(&dna, &name).ok_or(BlendError::DnaStructNotFound { name })?;
		let type_name = dna.type_name(item.type_idx);
		println!("struct: {}", type_name);
		println!("sdna_index: {}", sdna_idx);
		println!("field_count: {}", item.fields.len());
		for field in &item.fields {
			println!("  {} {}", dna.type_name(field.type_idx), dna.field_name(field.name_idx));
		}
	}

	Ok(())
}

fn find_struct_by_name<'a>(dna: &'a blendoc::blend::Dna, name: &str) -> Option<(usize, &'a DnaStruct)> {
	dna.structs.iter().enumerate().find(|(_, item)| dna.type_name(item.type_idx) == name)
}
