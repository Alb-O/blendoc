use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::{fs, io};

use clap::Parser;

const FIXTURE_MAP_FILE: &str = "blendfiles_map.json";

type FixtureMap = BTreeMap<String, BTreeMap<String, String>>;
type DynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Parser, Debug)]
#[command(name = "blendoc_blendfiles_downloader")]
#[command(about = "Download fixture files listed in fixtures/blendfiles/blendfiles_map.json")]
struct Args {
	/// Fixture root directory. `blendfiles` resolves to `fixtures/blendfiles`.
	#[arg(long, default_value = "fixtures/blendfiles")]
	root: PathBuf,
	/// Restrict downloads to one top-level fixture folder.
	#[arg(long)]
	folder: Option<String>,
	/// Print planned downloads without performing network requests.
	#[arg(long)]
	dry_run: bool,
	/// Re-download files even when the target already exists.
	#[arg(long)]
	overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DownloadItem {
	folder: String,
	relative_path: String,
	url: String,
	destination: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadOutcome {
	Downloaded,
	SkippedExists,
}

fn main() {
	if let Err(err) = run(Args::parse()) {
		eprintln!("error: {err}");
		std::process::exit(1);
	}
}

fn run(args: Args) -> Result<(), DynError> {
	let root = resolve_root(&args.root);
	let map_path = root.join(FIXTURE_MAP_FILE);
	let fixtures = load_fixture_map(&map_path)?;

	if let Some(folder) = args.folder.as_deref()
		&& !fixtures.contains_key(folder)
	{
		return Err(format!("folder '{folder}' not found in {}", map_path.display()).into());
	}

	let plan = build_download_plan(&fixtures, &root, args.folder.as_deref())?;
	if plan.is_empty() {
		println!("No files matched the requested filters.");
		return Ok(());
	}

	if args.dry_run {
		println!("Dry run: {} file(s) planned into {}", plan.len(), root.display());
		for item in &plan {
			println!("PLAN\t{}\t<=\t{}", item.destination.display(), item.url);
		}
		return Ok(());
	}

	let mut downloaded = 0usize;
	let mut skipped = 0usize;

	for item in plan {
		match download_file(&item, args.overwrite) {
			Ok(DownloadOutcome::Downloaded) => {
				downloaded += 1;
				println!("GET\t{}\t<=\t{}", item.destination.display(), item.url);
			}
			Ok(DownloadOutcome::SkippedExists) => {
				skipped += 1;
				println!("SKIP\t{}", item.destination.display());
			}
			Err(err) => {
				return Err(format!("failed downloading {} to {}: {err}", item.url, item.destination.display()).into());
			}
		}
	}

	println!(
		"Done. downloaded={} skipped={} total={} root={}",
		downloaded,
		skipped,
		downloaded + skipped,
		root.display()
	);

	Ok(())
}

fn load_fixture_map(map_path: &Path) -> Result<FixtureMap, DynError> {
	let bytes = fs::read(map_path)?;
	let map = serde_json::from_slice::<FixtureMap>(&bytes)?;
	Ok(map)
}

fn build_download_plan(fixtures: &FixtureMap, root: &Path, folder_filter: Option<&str>) -> Result<Vec<DownloadItem>, DynError> {
	let mut plan = Vec::new();

	for (folder, entries) in fixtures {
		if let Some(filter) = folder_filter
			&& filter != folder
		{
			continue;
		}

		for (relative_path, url) in entries {
			validate_relative_path(relative_path)?;
			plan.push(DownloadItem {
				folder: folder.clone(),
				relative_path: relative_path.clone(),
				url: url.clone(),
				destination: root.join(folder).join(relative_path),
			});
		}
	}

	Ok(plan)
}

fn validate_relative_path(path: &str) -> Result<(), DynError> {
	if path.is_empty() {
		return Err("empty relative path in fixture map".into());
	}

	let candidate = Path::new(path);
	if candidate.is_absolute() {
		return Err(format!("absolute relative path '{path}' is not allowed").into());
	}

	for component in candidate.components() {
		match component {
			Component::CurDir | Component::Normal(_) => {}
			Component::ParentDir => return Err(format!("parent path traversal '{path}' is not allowed").into()),
			Component::RootDir | Component::Prefix(_) => return Err(format!("rooted path '{path}' is not allowed").into()),
		}
	}

	Ok(())
}

fn download_file(item: &DownloadItem, overwrite: bool) -> Result<DownloadOutcome, DynError> {
	if item.destination.exists() && !overwrite {
		return Ok(DownloadOutcome::SkippedExists);
	}

	if let Some(parent) = item.destination.parent() {
		fs::create_dir_all(parent)?;
	}

	run_download_command(&item.url, &item.destination)?;

	Ok(DownloadOutcome::Downloaded)
}

fn run_download_command(url: &str, destination: &Path) -> Result<(), DynError> {
	let curl_status = Command::new("curl")
		.arg("--fail")
		.arg("--location")
		.arg("--silent")
		.arg("--show-error")
		.arg("--output")
		.arg(destination)
		.arg(url)
		.status();

	match curl_status {
		Ok(status) if status.success() => Ok(()),
		Ok(status) => Err(format!("curl exited with status {status} for {url}").into()),
		Err(err) if err.kind() == io::ErrorKind::NotFound => {
			let wget_status = Command::new("wget").arg("--quiet").arg("--output-document").arg(destination).arg(url).status();

			match wget_status {
				Ok(status) if status.success() => Ok(()),
				Ok(status) => Err(format!("wget exited with status {status} for {url}").into()),
				Err(err) if err.kind() == io::ErrorKind::NotFound => Err("neither `curl` nor `wget` is available; install one to download fixtures".into()),
				Err(err) => Err(format!("failed to run wget: {err}").into()),
			}
		}
		Err(err) => Err(format!("failed to run curl: {err}").into()),
	}
}

fn resolve_root(input: &Path) -> PathBuf {
	if input.is_absolute() {
		return input.to_path_buf();
	}

	let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
	let workspace = workspace_root();

	let candidates = [cwd.join(input), workspace.join(input), workspace.join("fixtures").join(input)];
	for candidate in candidates {
		if candidate.exists() {
			return candidate;
		}
	}

	if input.starts_with("fixtures") {
		workspace.join(input)
	} else {
		workspace.join("fixtures").join(input)
	}
}

fn workspace_root() -> PathBuf {
	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
	let candidate = manifest_dir.join("..").join("..");
	fs::canonicalize(&candidate).unwrap_or(candidate)
}

#[cfg(test)]
mod tests;
