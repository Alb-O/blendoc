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
	Ids {
		path: PathBuf,
		#[arg(long)]
		code: Option<String>,
		#[arg(long = "type")]
		type_name: Option<String>,
		#[arg(long)]
		limit: Option<usize>,
		#[arg(long)]
		json: bool,
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
	Chase {
		file: PathBuf,
		#[arg(long)]
		code: Option<String>,
		#[arg(long)]
		ptr: Option<String>,
		#[arg(long = "id")]
		id_name: Option<String>,
		#[arg(long = "path")]
		path_expr: String,
		#[arg(long)]
		json: bool,
	},
	Refs {
		file: PathBuf,
		#[arg(long)]
		code: Option<String>,
		#[arg(long)]
		ptr: Option<String>,
		#[arg(long = "id")]
		id_name: Option<String>,
		#[arg(long)]
		depth: Option<u32>,
		#[arg(long)]
		limit: Option<usize>,
		#[arg(long)]
		json: bool,
	},
	Graph {
		file: PathBuf,
		#[arg(long)]
		code: Option<String>,
		#[arg(long)]
		ptr: Option<String>,
		#[arg(long = "id")]
		id_name: Option<String>,
		#[arg(long)]
		depth: Option<u32>,
		#[arg(long = "refs-depth")]
		refs_depth: Option<u32>,
		#[arg(long = "max-nodes")]
		max_nodes: Option<usize>,
		#[arg(long = "max-edges")]
		max_edges: Option<usize>,
		#[arg(long = "id-only")]
		id_only: bool,
		#[arg(long)]
		dot: bool,
		#[arg(long)]
		json: bool,
	},
	Idgraph {
		file: PathBuf,
		#[arg(long = "refs-depth")]
		refs_depth: Option<u32>,
		#[arg(long = "max-edges")]
		max_edges: Option<usize>,
		#[arg(long)]
		dot: bool,
		#[arg(long)]
		json: bool,
		#[arg(long)]
		prefix: Option<String>,
		#[arg(long = "type")]
		type_name: Option<String>,
	},
	Xref {
		file: PathBuf,
		#[arg(long = "id")]
		id_name: Option<String>,
		#[arg(long)]
		ptr: Option<String>,
		#[arg(long = "refs-depth")]
		refs_depth: Option<u32>,
		#[arg(long)]
		limit: Option<usize>,
		#[arg(long)]
		json: bool,
	},
	Route {
		file: PathBuf,
		#[arg(long = "from-id")]
		from_id: Option<String>,
		#[arg(long = "from-ptr")]
		from_ptr: Option<String>,
		#[arg(long = "from-code")]
		from_code: Option<String>,
		#[arg(long = "to-id")]
		to_id: Option<String>,
		#[arg(long = "to-ptr")]
		to_ptr: Option<String>,
		#[arg(long)]
		depth: Option<u32>,
		#[arg(long = "refs-depth")]
		refs_depth: Option<u32>,
		#[arg(long = "max-nodes")]
		max_nodes: Option<usize>,
		#[arg(long = "max-edges")]
		max_edges: Option<usize>,
		#[arg(long)]
		json: bool,
	},
	Show {
		file: PathBuf,
		#[arg(long = "id")]
		id_name: Option<String>,
		#[arg(long)]
		ptr: Option<String>,
		#[arg(long)]
		code: Option<String>,
		#[arg(long = "path")]
		path_expr: Option<String>,
		#[arg(long)]
		trace: bool,
		#[arg(long)]
		json: bool,
		#[arg(long = "max-depth")]
		max_depth: Option<u32>,
		#[arg(long = "max-array")]
		max_array: Option<usize>,
		#[arg(long = "include-padding")]
		include_padding: bool,
		#[arg(long = "strict-layout")]
		strict_layout: bool,
		#[arg(long = "annotate-ptrs", default_value_t = true)]
		annotate_ptrs: bool,
		#[arg(long = "raw-ptrs")]
		raw_ptrs: bool,
		#[arg(long = "expand-depth", default_value_t = 0)]
		expand_depth: u32,
		#[arg(long = "expand-max-nodes", default_value_t = 64)]
		expand_max_nodes: usize,
	},
	Walk {
		file: PathBuf,
		#[arg(long = "id")]
		id_name: Option<String>,
		#[arg(long)]
		ptr: Option<String>,
		#[arg(long)]
		code: Option<String>,
		#[arg(long = "path")]
		path_expr: Option<String>,
		#[arg(long = "next", default_value = "next")]
		next_field: String,
		#[arg(long = "refs-depth")]
		refs_depth: Option<u32>,
		#[arg(long = "limit")]
		limit: Option<usize>,
		#[arg(long)]
		json: bool,
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
		Commands::Ids {
			path,
			code,
			type_name,
			limit,
			json,
		} => cmd::ids::run(path, code, type_name, limit, json),
		Commands::Dna { path, struct_name } => cmd::dna::run(path, struct_name),
		Commands::Decode { path, code } => cmd::decode::run(path, code),
		Commands::Chase {
			file,
			code,
			ptr,
			id_name,
			path_expr,
			json,
		} => cmd::chase::run(file, code, ptr, id_name, path_expr, json),
		Commands::Refs {
			file,
			code,
			ptr,
			id_name,
			depth,
			limit,
			json,
		} => cmd::refs::run(file, code, ptr, id_name, depth, limit, json),
		Commands::Graph {
			file,
			code,
			ptr,
			id_name,
			depth,
			refs_depth,
			max_nodes,
			max_edges,
			id_only,
			dot,
			json,
		} => cmd::graph::run(cmd::graph::GraphArgs {
			path: file,
			code,
			ptr,
			id_name,
			depth,
			refs_depth,
			max_nodes,
			max_edges,
			id_only,
			dot,
			json,
		}),
		Commands::Idgraph {
			file,
			refs_depth,
			max_edges,
			dot,
			json,
			prefix,
			type_name,
		} => cmd::idgraph::run(file, refs_depth, max_edges, dot, json, prefix, type_name),
		Commands::Xref {
			file,
			id_name,
			ptr,
			refs_depth,
			limit,
			json,
		} => cmd::xref::run(file, id_name, ptr, refs_depth, limit, json),
		Commands::Route {
			file,
			from_id,
			from_ptr,
			from_code,
			to_id,
			to_ptr,
			depth,
			refs_depth,
			max_nodes,
			max_edges,
			json,
		} => cmd::route::run(file, from_id, from_ptr, from_code, to_id, to_ptr, depth, refs_depth, max_nodes, max_edges, json),
		Commands::Show {
			file,
			id_name,
			ptr,
			code,
			path_expr,
			trace,
			json,
			max_depth,
			max_array,
			include_padding,
			strict_layout,
			annotate_ptrs,
			raw_ptrs,
			expand_depth,
			expand_max_nodes,
		} => cmd::show::run(
			file,
			id_name,
			ptr,
			code,
			path_expr,
			trace,
			json,
			max_depth,
			max_array,
			include_padding,
			strict_layout,
			annotate_ptrs,
			raw_ptrs,
			expand_depth,
			expand_max_nodes,
		),
		Commands::Walk {
			file,
			id_name,
			ptr,
			code,
			path_expr,
			next_field,
			refs_depth,
			limit,
			json,
		} => cmd::walk::run(cmd::walk::WalkArgs {
			path: file,
			id_name,
			ptr,
			code,
			path_expr,
			next_field,
			refs_depth,
			limit,
			json,
		}),
		Commands::Scene { path } => cmd::scene::run(path),
		Commands::Camera { path } => cmd::camera::run(path),
	}
}
