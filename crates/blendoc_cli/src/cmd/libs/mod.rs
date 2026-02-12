use std::path::PathBuf;

use blendoc::blend::{BlendFile, LinkSignal, scan_id_link_provenance, scan_library_records};

use crate::cmd::util::{emit_json, ptr_hex};

#[derive(clap::Args)]
pub struct Args {
	pub file: PathBuf,
	#[arg(long)]
	pub linked_only: bool,
	#[arg(long)]
	pub limit: Option<usize>,
	#[arg(long)]
	pub json: bool,
}

/// Scan and print linked-library declarations and per-ID link provenance.
pub fn run(args: Args) -> blendoc::blend::Result<()> {
	let Args {
		file: path,
		linked_only,
		limit,
		json,
	} = args;

	let blend = BlendFile::open(&path)?;
	let dna = blend.dna()?;
	let libraries = scan_library_records(&blend, &dna)?;
	let mut ids = scan_id_link_provenance(&blend, &dna)?;

	if linked_only {
		ids.retain(|item| item.linked);
	}
	if let Some(limit) = limit {
		ids.truncate(limit);
	}

	if json {
		let payload = LibsJson {
			path: path.display().to_string(),
			libraries: libraries
				.iter()
				.map(|item| LibraryJson {
					id_name: item.id_name.to_string(),
					id_ptr: ptr_hex(item.id_ptr),
					library_path: item.library_path.to_string(),
					is_relative: item.is_relative,
				})
				.collect(),
			ids: ids
				.iter()
				.map(|item| IdProvenanceJson {
					id_name: item.id_name.to_string(),
					type_name: item.type_name.to_string(),
					id_ptr: ptr_hex(item.id_ptr),
					linked: item.linked,
					confidence: item.confidence.as_str().to_owned(),
					signals: item.signals.iter().map(signal_to_json).collect(),
				})
				.collect(),
		};
		emit_json(&payload);
		return Ok(());
	}

	println!("path: {}", path.display());
	println!("libraries: {}", libraries.len());
	println!("ids: {}", ids.len());
	println!();
	println!("libraries:");
	println!("id_name\tid_ptr\tpath\trelative");
	for item in &libraries {
		println!("{}\t{}\t{}\t{}", item.id_name, ptr_hex(item.id_ptr), item.library_path, item.is_relative);
	}
	println!();
	println!("id_provenance:");
	println!("id_name\ttype\tid_ptr\tlinked\tconfidence\tsignals");
	for item in &ids {
		let signals = format_signal_summary(&item.signals);
		println!(
			"{}\t{}\t{}\t{}\t{}\t{}",
			item.id_name,
			item.type_name,
			ptr_hex(item.id_ptr),
			item.linked,
			item.confidence.as_str(),
			signals
		);
	}

	Ok(())
}

fn format_signal_summary(signals: &[LinkSignal]) -> String {
	if signals.is_empty() {
		return "-".to_owned();
	}

	signals
		.iter()
		.map(|signal| match signal {
			LinkSignal::IdLibPtr { ptr } => format!("id.lib={}", ptr_hex(*ptr)),
			LinkSignal::OverrideLibraryPtr { ptr } => format!("id.override_library={}", ptr_hex(*ptr)),
			LinkSignal::LibraryWeakReferencePtr { ptr } => format!("id.library_weak_reference={}", ptr_hex(*ptr)),
			LinkSignal::LibraryIdPresent { library_id_name, library_path } => format!("library={library_id_name}({library_path})"),
		})
		.collect::<Vec<_>>()
		.join(", ")
}

fn signal_to_json(signal: &LinkSignal) -> SignalJson {
	match signal {
		LinkSignal::IdLibPtr { ptr } => SignalJson {
			kind: "id_lib_ptr".to_owned(),
			ptr: Some(ptr_hex(*ptr)),
			library_id_name: None,
			library_path: None,
		},
		LinkSignal::OverrideLibraryPtr { ptr } => SignalJson {
			kind: "override_library_ptr".to_owned(),
			ptr: Some(ptr_hex(*ptr)),
			library_id_name: None,
			library_path: None,
		},
		LinkSignal::LibraryWeakReferencePtr { ptr } => SignalJson {
			kind: "library_weak_reference_ptr".to_owned(),
			ptr: Some(ptr_hex(*ptr)),
			library_id_name: None,
			library_path: None,
		},
		LinkSignal::LibraryIdPresent { library_id_name, library_path } => SignalJson {
			kind: "library_id_present".to_owned(),
			ptr: None,
			library_id_name: Some(library_id_name.to_string()),
			library_path: Some(library_path.to_string()),
		},
	}
}

#[derive(serde::Serialize)]
struct LibsJson {
	path: String,
	libraries: Vec<LibraryJson>,
	ids: Vec<IdProvenanceJson>,
}

#[derive(serde::Serialize)]
struct LibraryJson {
	id_name: String,
	id_ptr: String,
	library_path: String,
	is_relative: bool,
}

#[derive(serde::Serialize)]
struct IdProvenanceJson {
	id_name: String,
	#[serde(rename = "type")]
	type_name: String,
	id_ptr: String,
	linked: bool,
	confidence: String,
	signals: Vec<SignalJson>,
}

#[derive(serde::Serialize)]
struct SignalJson {
	kind: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	ptr: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	library_id_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	library_path: Option<String>,
}

#[cfg(test)]
mod tests;
