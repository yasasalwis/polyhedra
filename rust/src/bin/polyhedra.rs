//! # polyhedra CLI
//!
//! Compiles `.polyh` files to 3D mesh files (STL, OBJ, GLTF, PLY).
//! Built as a standalone Rust binary — no Python dependency.
//!
//! Usage:
//!   polyhedra -c main.polyh -o stl
//!   polyhedra -c main.polyh -o stl,gltf --quality high
//!   polyhedra -c main.polyh --validate
//!   polyhedra -c main.polyh -o stl --watch

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

// ── CLI argument definition ───────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name    = "polyhedra",
    version = env!("CARGO_PKG_VERSION"),
    about   = "polyhedra 3D compiler — compile .polyh files into 3D mesh files",
    long_about = "\
polyhedra compiles plain-English .polyh design files into 3D geometry.

Examples:
  polyhedra -c main.polyh -o stl
  polyhedra -c main.polyh -o stl,obj,gltf --quality high --dir ./output
  polyhedra -c main.polyh --validate
  polyhedra -c main.polyh -o stl --watch
  polyhedra --list bracket.polyh
"
)]
struct Cli {
    /// Input file to compile. Must be named main.polyh (the entry point).
    #[arg(short = 'c', long, value_name = "FILE", help = "Input .polyh file")]
    compile: Option<PathBuf>,

    /// Output format(s). Comma-separated list: stl, obj, gltf, ply, all.
    #[arg(
        short = 'o',
        long,
        value_name = "FORMAT",
        help = "Output format(s): stl,obj,gltf,ply,all"
    )]
    output: Option<String>,

    /// Mesh quality preset.
    #[arg(
        short = 'q',
        long,
        value_name = "QUALITY",
        default_value = "medium",
        help = "Mesh quality: low|medium|high|ultra"
    )]
    quality: Quality,

    /// Output directory. Defaults to the directory of the input file.
    #[arg(
        short = 'd',
        long,
        value_name = "DIR",
        help = "Output directory (default: same as input file)"
    )]
    dir: Option<PathBuf>,

    /// Validate syntax only — no geometry compilation.
    /// Runs instantly and reports all parse/variable errors.
    #[arg(long, help = "Validate syntax only, do not compile geometry")]
    validate: bool,

    /// Watch mode — recompile automatically when any .polyh file changes.
    #[arg(
        short = 'w',
        long,
        help = "Recompile on file changes (Ctrl+C to stop)"
    )]
    watch: bool,

    /// List all named objects (define blocks) in a .polyh file.
    #[arg(long, value_name = "FILE", help = "List all defined objects in a file")]
    list: Option<PathBuf>,

    /// Print resolved variable values and the import dependency graph.
    #[arg(long, help = "Print resolved variables and import graph")]
    debug_vars: bool,

    /// Print compilation statistics (time, triangle count, memory used).
    #[arg(long, help = "Print compile time, triangle count, memory")]
    stats: bool,

    /// Number of CPU threads for parallel SDF evaluation.
    /// Defaults to the number of logical cores.
    #[arg(
        short = 'j',
        long,
        value_name = "N",
        help = "CPU thread count (default: all cores)"
    )]
    threads: Option<usize>,
}

#[derive(ValueEnum, Clone, Debug)]
enum Quality {
    /// Fast preview mesh (refinement ≈ 0.1).
    Low,
    /// Good balance of speed and quality (refinement ≈ 0.01).
    Medium,
    /// High-quality output suitable for 3D printing (refinement ≈ 0.001).
    High,
    /// Maximum quality for engineering review (refinement ≈ 0.0001).
    Ultra,
}

impl Quality {
    fn refinement(&self) -> f32 {
        match self {
            Quality::Low    => 0.1,
            Quality::Medium => 0.01,
            Quality::High   => 0.001,
            Quality::Ultra  => 0.0001,
        }
    }
}

// ── Main ──────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    // ── Compile mode ──────────────────────────────────────────────────────
    if let Some(ref file) = cli.compile {
        let formats: Vec<&str> = cli
            .output
            .as_deref()
            .unwrap_or("stl")
            .split(',')
            .map(str::trim)
            .collect();

        let output_dir = cli
            .dir
            .clone()
            .or_else(|| file.parent().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));

        if cli.validate {
            println!("polyhedra: validating {} ...", file.display());
            println!("  Parser not yet implemented (Phase 2).");
            println!("  ✓ CLI argument parsing: OK");
            return;
        }

        println!("polyhedra v{}", env!("CARGO_PKG_VERSION"));
        println!("  input   : {}", file.display());
        println!("  output  : {}", formats.join(", "));
        println!("  quality : {:?} (refinement = {})", cli.quality, cli.quality.refinement());
        println!("  dir     : {}", output_dir.display());
        if let Some(j) = cli.threads {
            println!("  threads : {j}");
        }
        println!();

        if cli.watch {
            println!("  Watch mode enabled. File watching not yet implemented (Phase 15).");
            println!("  Ctrl+C to exit.");
            // Phase 15: integrate `notify` crate here.
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }

        println!("Geometry kernel not yet implemented (Phase 1+).");
        println!("Phase 0 scaffold: CLI parses all arguments correctly. ✓");
    }
    // ── List mode ─────────────────────────────────────────────────────────
    else if let Some(ref file) = cli.list {
        println!("polyhedra: listing objects in {} ...", file.display());
        println!("  Parser not yet implemented (Phase 2).");
    }
    // ── Debug vars mode ───────────────────────────────────────────────────
    else if cli.debug_vars {
        println!("polyhedra: debug-vars mode requires --compile (-c).");
    }
    // ── No command ────────────────────────────────────────────────────────
    else {
        eprintln!("No input file specified. Use --help for usage.");
        std::process::exit(1);
    }
}
