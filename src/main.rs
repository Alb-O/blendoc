#![allow(missing_docs)]

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
	Info(cmd::info::Args),
	Ids(cmd::ids::Args),
	Dna(cmd::dna::Args),
	Decode(cmd::decode::Args),
	Chase(cmd::chase::Args),
	Refs(cmd::refs::Args),
	Graph(cmd::graph::Args),
	Idgraph(cmd::idgraph::Args),
	Xref(cmd::xref::Args),
	Route(cmd::route::Args),
	Show(cmd::show::Args),
	Walk(cmd::walk::Args),
	Scene(cmd::scene::Args),
	Camera(cmd::camera::Args),
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
		Commands::Info(args) => cmd::info::run(args),
		Commands::Ids(args) => cmd::ids::run(args),
		Commands::Dna(args) => cmd::dna::run(args),
		Commands::Decode(args) => cmd::decode::run(args),
		Commands::Chase(args) => cmd::chase::run(args),
		Commands::Refs(args) => cmd::refs::run(args),
		Commands::Graph(args) => cmd::graph::run(args),
		Commands::Idgraph(args) => cmd::idgraph::run(args),
		Commands::Xref(args) => cmd::xref::run(args),
		Commands::Route(args) => cmd::route::run(args),
		Commands::Show(args) => cmd::show::run(args),
		Commands::Walk(args) => cmd::walk::run(args),
		Commands::Scene(args) => cmd::scene::run(args),
		Commands::Camera(args) => cmd::camera::run(args),
	}
}
