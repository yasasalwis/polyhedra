//! # polyhedra CLI
//!
//! Compiles `.polyh` files to 3D mesh files (STL, OBJ, GLB, PLY).
//! Built as a standalone Rust binary with no Python dependency.
//!
//! ```text
//! polyhedra -c main.polyh                        # → main.stl (default)
//! polyhedra -c main.polyh -o stl,glb             # multiple formats
//! polyhedra -c main.polyh -o stl --quality high  # fine mesh
//! polyhedra -c main.polyh --validate             # syntax check only
//! polyhedra -c main.polyh -o stl --watch         # auto-recompile on save
//! polyhedra --list bracket.polyh                 # list defined objects
//! polyhedra -c main.polyh -o stl --stats         # print timing + tri count
//! ```

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::{Parser, ValueEnum};
use notify::{Event, RecursiveMode, Watcher};

use _core::export::{ExportFormat, to_file};
use _core::mesher::{MeshConfig, mesh};
use _core::parser::{eval, parse_file};

// ── CLI argument definition ────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name    = "polyhedra",
    version = env!("CARGO_PKG_VERSION"),
    about   = "polyhedra 3D compiler — compile .polyh files into 3D mesh files",
    long_about = "\
polyhedra compiles plain-English .polyh design files into 3D geometry.

Examples:
  polyhedra -c main.polyh -o stl
  polyhedra -c main.polyh -o stl,obj,glb --quality high --dir ./output
  polyhedra -c main.polyh --validate
  polyhedra -c main.polyh -o stl --watch
  polyhedra --list bracket.polyh
"
)]
struct Cli {
    /// Input .polyh file to compile.
    #[arg(short = 'c', long, value_name = "FILE")]
    compile: Option<PathBuf>,

    /// Output format(s). Comma-separated: stl, obj, glb, ply, all.
    /// Defaults to stl.
    #[arg(short = 'o', long, value_name = "FORMAT")]
    output: Option<String>,

    /// Mesh quality preset (controls voxel resolution).
    #[arg(short = 'q', long, value_name = "QUALITY", default_value = "medium")]
    quality: Quality,

    /// Output directory. Defaults to the directory of the input file.
    #[arg(short = 'd', long, value_name = "DIR")]
    dir: Option<PathBuf>,

    /// Validate syntax only — do not compile geometry.
    #[arg(long)]
    validate: bool,

    /// Watch mode — recompile automatically when any .polyh file changes.
    #[arg(short = 'w', long)]
    watch: bool,

    /// List all named objects (define blocks) in a .polyh file.
    #[arg(long, value_name = "FILE")]
    list: Option<PathBuf>,

    /// Print compilation statistics (time, triangle count, bounds).
    #[arg(long)]
    stats: bool,

    /// Override thread count for parallel SDF evaluation.
    /// Defaults to all logical cores.
    #[arg(short = 'j', long, value_name = "N")]
    threads: Option<usize>,
}

#[derive(ValueEnum, Clone, Debug)]
enum Quality {
    /// Fast preview — 32 voxels per axis.
    Low,
    /// Balanced quality — 64 voxels per axis (default).
    Medium,
    /// High quality — 128 voxels per axis.
    High,
    /// Ultra — 256 voxels per axis (slow on large models).
    Ultra,
}

impl Quality {
    fn resolution(&self) -> u32 {
        match self {
            Quality::Low => 32,
            Quality::Medium => 64,
            Quality::High => 128,
            Quality::Ultra => 256,
        }
    }
}

// ── Main ───────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    // Configure rayon thread pool if requested.
    if let Some(n) = cli.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .expect("failed to configure thread pool");
    }

    // Dispatch to the appropriate mode.
    let result = if let Some(ref file) = cli.list {
        run_list(file)
    } else if let Some(ref file) = cli.compile {
        if cli.validate {
            run_validate(file)
        } else if cli.watch {
            run_watch(file, &cli)
        } else {
            run_compile(file, &cli)
        }
    } else {
        eprintln!("polyhedra: no input file. Use --help for usage.");
        std::process::exit(1);
    };

    if let Err(e) = result {
        eprintln!("polyhedra: error: {e}");
        std::process::exit(1);
    }
}

// ── Compile ────────────────────────────────────────────────────────────────────

fn run_compile(file: &Path, cli: &Cli) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let formats = resolve_formats(cli.output.as_deref().unwrap_or("stl"))?;
    let out_dir = resolve_output_dir(cli.dir.as_deref(), file);

    println!("polyhedra v{}", env!("CARGO_PKG_VERSION"));
    println!("  input   : {}", file.display());
    println!(
        "  output  : {}",
        formats
            .iter()
            .map(|f| f.extension())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "  quality : {:?} ({}vox)",
        cli.quality,
        cli.quality.resolution()
    );
    println!("  dir     : {}", out_dir.display());
    println!();

    compile_once(
        file,
        &formats,
        &out_dir,
        cli.quality.resolution(),
        cli.stats,
    )
}

fn compile_once(
    file: &Path,
    formats: &[ExportFormat],
    out_dir: &Path,
    resolution: u32,
    print_stats: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();

    // ── Parse ─────────────────────────────────────────────────────────────────
    print!("  parsing  {} … ", file.display());
    let polyh = parse_file(file)?;
    println!("ok");

    // ── Evaluate → SDF ────────────────────────────────────────────────────────
    print!("  building SDF tree … ");
    let (sdf_node, bounds) = if polyh.assemblies().next().is_some() {
        let defines: std::collections::HashMap<String, &_core::parser::ast::DefineBlock> =
            polyh.defines().map(|d| (d.name.clone(), d)).collect();
        let asm = polyh.assemblies().next().unwrap();
        eval::eval_assemble(asm, &defines, 1.0)?
    } else {
        let def = polyh
            .defines()
            .next()
            .ok_or("file has no 'define' or 'assemble' block")?;
        eval::eval_define(def, 1.0)?
    };
    println!("ok  (bounds ≈ {bounds:.1} mm)");

    // ── Mesh ──────────────────────────────────────────────────────────────────
    print!(
        "  meshing  ({}vox, bounds {:.0}mm) … ",
        resolution,
        bounds * 1.1
    );
    let cfg = MeshConfig::centered(bounds * 1.1, resolution);
    let m = mesh(sdf_node.as_ref(), &cfg);
    println!("{} triangles", m.triangle_count());

    if m.triangle_count() == 0 {
        return Err("mesh is empty — try increasing quality or check that the \
                    geometry fits within the bounds"
            .into());
    }

    // ── Export ────────────────────────────────────────────────────────────────
    let stem = file.file_stem().unwrap_or_default().to_string_lossy();
    for fmt in formats {
        let out_path = out_dir.join(format!("{stem}.{}", fmt.extension()));
        print!("  writing  {} … ", out_path.display());
        to_file(&m, &out_path)?;
        let size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
        println!("{} bytes", size);
    }

    if print_stats {
        let elapsed = t0.elapsed();
        println!();
        println!("  stats:");
        println!("    triangles : {}", m.triangle_count());
        println!("    vertices  : {}", m.vertex_count());
        println!("    bounds    : ±{bounds:.1} mm");
        println!("    time      : {:.2}s", elapsed.as_secs_f64());
    }

    println!();
    println!("  done ✓");
    Ok(())
}

// ── Validate ───────────────────────────────────────────────────────────────────

fn run_validate(file: &Path) -> std::result::Result<(), Box<dyn std::error::Error>> {
    print!("polyhedra: validating {} … ", file.display());
    match parse_file(file) {
        Ok(polyh) => {
            let n_defs = polyh.defines().count();
            let n_asms = polyh.assemblies().count();
            let has_export = polyh.export().is_some();
            println!("ok");
            println!("  define blocks  : {n_defs}");
            println!("  assemble blocks: {n_asms}");
            println!(
                "  export block   : {}",
                if has_export { "yes" } else { "no" }
            );
            println!();
            println!("  ✓ Syntax valid");
            Ok(())
        }
        Err(e) => {
            println!("FAILED");
            Err(e.into())
        }
    }
}

// ── List ───────────────────────────────────────────────────────────────────────

fn run_list(file: &Path) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let polyh = parse_file(file)?;
    println!("Objects defined in {}:", file.display());
    let mut count = 0;
    for def in polyh.defines() {
        let prim_count = def
            .items
            .iter()
            .filter(|i| matches!(i, _core::parser::ast::DefineItem::Primitive(_)))
            .count();
        let has_manip = def
            .items
            .iter()
            .any(|i| matches!(i, _core::parser::ast::DefineItem::Manip(_)));
        print!("  define {}", def.name);
        if prim_count > 0 {
            print!("  ({prim_count} primitive");
            if prim_count != 1 {
                print!("s");
            }
            print!(")");
        }
        if has_manip {
            print!("  [manipulations]");
        }
        println!();
        count += 1;
    }
    for asm in polyh.assemblies() {
        let n_ops = asm
            .items
            .iter()
            .filter(|i| matches!(i, _core::parser::ast::AssembleItem::Op(_)))
            .count();
        println!("  assemble {}  ({n_ops} operations)", asm.name);
        count += 1;
    }
    if count == 0 {
        println!("  (no objects found)");
    }
    Ok(())
}

// ── Watch ──────────────────────────────────────────────────────────────────────

fn run_watch(file: &Path, cli: &Cli) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let formats = resolve_formats(cli.output.as_deref().unwrap_or("stl"))?;
    let out_dir = resolve_output_dir(cli.dir.as_deref(), file);
    let res = cli.quality.resolution();
    let abs_file = file.canonicalize()?;
    let watch_dir = abs_file.parent().unwrap_or(Path::new(".")).to_path_buf();

    println!(
        "polyhedra watch — watching {} for changes (Ctrl+C to stop)",
        watch_dir.display()
    );
    println!();

    // Run once immediately.
    let _ = compile_once(&abs_file, &formats, &out_dir, res, cli.stats);

    // Set up file watcher.
    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(&watch_dir, RecursiveMode::Recursive)?;

    let mut last_event = Instant::now();

    for event in rx {
        match event {
            Ok(evt) => {
                // Filter to .polyh files; debounce to 200ms.
                let is_polyh = evt
                    .paths
                    .iter()
                    .any(|p| p.extension().map(|e| e == "polyh").unwrap_or(false));
                if is_polyh && last_event.elapsed().as_millis() > 200 {
                    last_event = Instant::now();
                    println!("--- change detected — recompiling ---");
                    let _ = compile_once(&abs_file, &formats, &out_dir, res, cli.stats);
                }
            }
            Err(e) => eprintln!("watch error: {e}"),
        }
    }

    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn resolve_formats(
    spec: &str,
) -> std::result::Result<Vec<ExportFormat>, Box<dyn std::error::Error>> {
    const ALL: &[ExportFormat] = &[
        ExportFormat::Stl,
        ExportFormat::Obj,
        ExportFormat::Ply,
        ExportFormat::Glb,
    ];
    if spec.eq_ignore_ascii_case("all") {
        return Ok(ALL.to_vec());
    }
    let mut seen = HashSet::new();
    let mut fmts = Vec::new();
    for part in spec.split(',') {
        let s = part.trim();
        if s.is_empty() {
            continue;
        }
        let fmt: ExportFormat = s
            .parse()
            .map_err(|_| format!("unknown format '{s}'. Valid: stl, obj, glb, ply, all"))?;
        if seen.insert(fmt) {
            fmts.push(fmt);
        }
    }
    if fmts.is_empty() {
        return Err("no output formats specified".into());
    }
    Ok(fmts)
}

fn resolve_output_dir(dir: Option<&Path>, input: &Path) -> PathBuf {
    dir.map(PathBuf::from).unwrap_or_else(|| {
        input
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    })
}
