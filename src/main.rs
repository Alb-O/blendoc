#![allow(missing_docs)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod cmd;

#[derive(Parser)]
#[command(name = "blendoc", about = "Blender .blend inspection tools")]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Info {
		path: PathBuf,
	},
	Dna {
		path: PathBuf,
		#[arg(long = "struct")]
		struct_name: Option<String>,
	},
	Decode {
		path: PathBuf,
		#[arg(long)]
		code: String,
	},
	Scene {
		path: PathBuf,
	},
	Camera {
		path: PathBuf,
	},
}

fn main() {
	if let Err(err) = run() {
		eprintln!("error: {err}");
		std::process::exit(1);
	}
}

fn run() -> blendoc::blend::Result<()> {
	let cli = Cli::parse();

	match cli.command {
		Commands::Info { path } => cmd::info::run(path),
		Commands::Dna { path, struct_name } => cmd::dna::run(path, struct_name),
		Commands::Decode { path, code } => cmd::decode::run(path, code),
		Commands::Scene { path } => cmd::scene::run(path),
		Commands::Camera { path } => cmd::camera::run(path),
	}
}
