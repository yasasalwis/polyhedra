//! End-to-end CLI integration tests.
//!
//! These tests compile the `polyhedra` binary and run it as a child process,
//! verifying exit codes, stdout patterns, and produced output files.

use std::path::{Path, PathBuf};
use std::process::Command;

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Path to the compiled `polyhedra` binary (built by cargo before tests run).
fn bin_path() -> PathBuf {
    // CARGO_BIN_EXE_polyhedra is set by cargo test when a [[bin]] target exists.
    // Fall back to the default debug build path if the env var is absent.
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_polyhedra") {
        PathBuf::from(p)
    } else {
        // workspace root / target / debug / polyhedra
        let manifest = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        manifest
            .parent() // workspace root
            .unwrap_or(&manifest)
            .join("target/debug/polyhedra")
    }
}

/// Directory containing the example .polyh files.
fn examples_dir() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    manifest.parent().unwrap_or(&manifest).join("examples")
}

/// Run the binary with `args`, returning `(exit_ok, stdout, stderr)`.
fn run(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(bin_path())
        .args(args)
        .output()
        .expect("failed to spawn polyhedra binary");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (output.status.success(), stdout, stderr)
}

/// Run the binary and also capture any produced file in `out_dir`.
fn run_with_outdir(args: &[&str], extra_args: &[(&str, &str)]) -> (bool, String, String) {
    let mut cmd = Command::new(bin_path());
    cmd.args(args);
    for (k, v) in extra_args {
        cmd.arg(k).arg(v);
    }
    let output = cmd.output().expect("failed to spawn polyhedra binary");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (output.status.success(), stdout, stderr)
}

// ── --help ─────────────────────────────────────────────────────────────────────

#[test]
fn help_exits_zero() {
    let (ok, out, _) = run(&["--help"]);
    assert!(ok, "help should exit 0");
    assert!(
        out.contains("polyhedra"),
        "help text should mention the tool name"
    );
}

#[test]
fn version_exits_zero() {
    let (ok, out, _) = run(&["--version"]);
    assert!(ok, "version should exit 0");
    assert!(
        out.contains("polyhedra"),
        "version text should include binary name"
    );
}

// ── No args → non-zero exit ───────────────────────────────────────────────────

#[test]
fn no_args_exits_nonzero() {
    let (ok, _, err) = run(&[]);
    assert!(!ok, "no args should exit non-zero");
    assert!(
        err.contains("no input file") || err.contains("Usage") || err.contains("help"),
        "stderr should mention how to get help: {err}"
    );
}

// ── --validate ─────────────────────────────────────────────────────────────────

#[test]
fn validate_hello_polyh_ok() {
    let hello = examples_dir().join("hello.polyh");
    let (ok, out, err) = run(&["-c", hello.to_str().unwrap(), "--validate"]);
    assert!(ok, "validate should succeed: stderr={err}");
    assert!(
        out.contains("valid") || out.contains("ok") || out.contains("✓"),
        "output should report success: {out}"
    );
}

#[test]
fn validate_missing_file_exits_nonzero() {
    let (ok, _, err) = run(&["-c", "/tmp/__does_not_exist_polyh__.polyh", "--validate"]);
    assert!(!ok, "missing file should exit non-zero");
    assert!(
        err.contains("error") || err.contains("not found") || !err.is_empty(),
        "stderr should have an error message: {err}"
    );
}

#[test]
fn validate_reports_define_and_assemble_counts() {
    let hello = examples_dir().join("hello.polyh");
    let (ok, out, _) = run(&["-c", hello.to_str().unwrap(), "--validate"]);
    assert!(ok);
    // hello.polyh has 2 define blocks and 0 assemble blocks
    assert!(
        out.contains("2") || out.contains("define"),
        "output should mention defines: {out}"
    );
}

// ── --list ─────────────────────────────────────────────────────────────────────

#[test]
fn list_hello_polyh_shows_defines() {
    let hello = examples_dir().join("hello.polyh");
    let (ok, out, err) = run(&["--list", hello.to_str().unwrap()]);
    assert!(ok, "list should succeed: stderr={err}");
    assert!(out.contains("bracket"), "should list 'bracket': {out}");
    assert!(out.contains("m3_hole"), "should list 'm3_hole': {out}");
}

#[test]
fn list_shows_primitive_count() {
    let hello = examples_dir().join("hello.polyh");
    let (ok, out, _) = run(&["--list", hello.to_str().unwrap()]);
    assert!(ok);
    // bracket has 2 cubes, m3_hole has 1 cylinder
    assert!(
        out.contains("primitive") || out.contains("1") || out.contains("2"),
        "should report primitive counts: {out}"
    );
}

#[test]
fn list_main_polyh_shows_assemble() {
    let main = examples_dir().join("main.polyh");
    let (ok, out, err) = run(&["--list", main.to_str().unwrap()]);
    assert!(ok, "list should succeed for main.polyh: stderr={err}");
    assert!(
        out.contains("assemble") || out.contains("bracket_with_holes"),
        "should list assemble block: {out}"
    );
}

// ── --compile ──────────────────────────────────────────────────────────────────

#[test]
fn compile_hello_produces_stl() {
    let hello = examples_dir().join("hello.polyh");
    let tmp = std::env::temp_dir().join("polyhedra_test_compile");
    std::fs::create_dir_all(&tmp).ok();

    let (ok, out, err) = run_with_outdir(
        &[
            "-c",
            hello.to_str().unwrap(),
            "-o",
            "stl",
            "--quality",
            "low",
        ],
        &[("-d", tmp.to_str().unwrap())],
    );
    assert!(ok, "compile should succeed: stderr={err}\nstdout={out}");

    let stl_path = tmp.join("hello.stl");
    assert!(
        stl_path.exists(),
        "STL output file should exist at {}",
        stl_path.display()
    );
    let size = std::fs::metadata(&stl_path).unwrap().len();
    assert!(
        size > 84,
        "STL should be larger than the 84-byte header: {size} bytes"
    );

    // STL triangle count at bytes 80-84 should be non-zero
    let data = std::fs::read(&stl_path).unwrap();
    let tri_count = u32::from_le_bytes([data[80], data[81], data[82], data[83]]);
    assert!(
        tri_count > 0,
        "STL should contain triangles, got {tri_count}"
    );

    // Clean up
    std::fs::remove_file(&stl_path).ok();
}

#[test]
fn compile_hello_produces_obj() {
    let hello = examples_dir().join("hello.polyh");
    let tmp = std::env::temp_dir().join("polyhedra_test_obj");
    std::fs::create_dir_all(&tmp).ok();

    let (ok, out, err) = run_with_outdir(
        &[
            "-c",
            hello.to_str().unwrap(),
            "-o",
            "obj",
            "--quality",
            "low",
        ],
        &[("-d", tmp.to_str().unwrap())],
    );
    assert!(
        ok,
        "compile to obj should succeed: stderr={err}\nstdout={out}"
    );

    let obj_path = tmp.join("hello.obj");
    assert!(obj_path.exists(), "OBJ output file should exist");
    let text = std::fs::read_to_string(&obj_path).unwrap();
    assert!(text.contains("v "), "OBJ should have vertices");
    assert!(text.contains("\nf "), "OBJ should have faces");

    std::fs::remove_file(&obj_path).ok();
}

#[test]
fn compile_hello_multiple_formats() {
    let hello = examples_dir().join("hello.polyh");
    let tmp = std::env::temp_dir().join("polyhedra_test_multi");
    std::fs::create_dir_all(&tmp).ok();

    let (ok, _, err) = run_with_outdir(
        &[
            "-c",
            hello.to_str().unwrap(),
            "-o",
            "stl,ply",
            "--quality",
            "low",
        ],
        &[("-d", tmp.to_str().unwrap())],
    );
    assert!(ok, "multi-format compile should succeed: {err}");
    assert!(tmp.join("hello.stl").exists(), "STL should exist");
    assert!(tmp.join("hello.ply").exists(), "PLY should exist");

    std::fs::remove_file(tmp.join("hello.stl")).ok();
    std::fs::remove_file(tmp.join("hello.ply")).ok();
}

#[test]
fn compile_all_format_alias() {
    let hello = examples_dir().join("hello.polyh");
    let tmp = std::env::temp_dir().join("polyhedra_test_all");
    std::fs::create_dir_all(&tmp).ok();

    let (ok, _, err) = run_with_outdir(
        &[
            "-c",
            hello.to_str().unwrap(),
            "-o",
            "all",
            "--quality",
            "low",
        ],
        &[("-d", tmp.to_str().unwrap())],
    );
    assert!(ok, "compile --output all should succeed: {err}");
    assert!(tmp.join("hello.stl").exists(), "STL should exist");
    assert!(tmp.join("hello.obj").exists(), "OBJ should exist");
    assert!(tmp.join("hello.ply").exists(), "PLY should exist");
    assert!(tmp.join("hello.glb").exists(), "GLB should exist");

    for ext in &["stl", "obj", "ply", "glb"] {
        std::fs::remove_file(tmp.join(format!("hello.{ext}"))).ok();
    }
}

#[test]
fn compile_with_stats_flag() {
    let hello = examples_dir().join("hello.polyh");
    let tmp = std::env::temp_dir().join("polyhedra_test_stats");
    std::fs::create_dir_all(&tmp).ok();

    let (ok, out, err) = run_with_outdir(
        &[
            "-c",
            hello.to_str().unwrap(),
            "-o",
            "stl",
            "--quality",
            "low",
            "--stats",
        ],
        &[("-d", tmp.to_str().unwrap())],
    );
    assert!(ok, "compile --stats should succeed: {err}");
    assert!(
        out.contains("triangles") && out.contains("time"),
        "stats output should include triangle count and time: {out}"
    );

    std::fs::remove_file(tmp.join("hello.stl")).ok();
}

#[test]
fn compile_invalid_format_exits_nonzero() {
    let hello = examples_dir().join("hello.polyh");
    let (ok, _, err) = run(&["-c", hello.to_str().unwrap(), "-o", "xyz"]);
    assert!(!ok, "unknown format should exit non-zero");
    assert!(
        err.contains("unknown format") || err.contains("error") || err.contains("xyz"),
        "error should mention the bad format: {err}"
    );
}

// ── Inline fixture tests ────────────────────────────────────────────────────────

/// Write a uniquely-named temp .polyh file and invoke `test` with its path.
/// Uses `name` to avoid collisions between parallel tests.
fn with_tmp_polyh(name: &str, src: &str, test: impl FnOnce(&Path)) {
    let dir = std::env::temp_dir().join(format!("polyhedra_fixture_{name}"));
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join(format!("{name}.polyh"));
    std::fs::write(&path, src).unwrap();
    test(&path);
    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&dir).ok();
}

#[test]
fn compile_sphere_fixture() {
    let src = "define ball\n  units mm\n  sphere\n    radius 5\n  end\nend\n";
    with_tmp_polyh("sphere_fix", src, |path| {
        let tmp = std::env::temp_dir().join("polyhedra_sphere_out");
        std::fs::create_dir_all(&tmp).ok();
        let (ok, _, err) = run_with_outdir(
            &[
                "-c",
                path.to_str().unwrap(),
                "-o",
                "stl",
                "--quality",
                "low",
            ],
            &[("-d", tmp.to_str().unwrap())],
        );
        assert!(ok, "sphere fixture should compile: {err}");
        assert!(tmp.join("sphere_fix.stl").exists());
        std::fs::remove_file(tmp.join("sphere_fix.stl")).ok();
    });
}

#[test]
fn compile_cylinder_fixture() {
    let src = "define rod\n  units mm\n  cylinder\n    radius 4\n    height 20\n  end\nend\n";
    with_tmp_polyh("cylinder_fix", src, |path| {
        let tmp = std::env::temp_dir().join("polyhedra_cylinder_out");
        std::fs::create_dir_all(&tmp).ok();
        let (ok, _, err) = run_with_outdir(
            &[
                "-c",
                path.to_str().unwrap(),
                "-o",
                "stl",
                "--quality",
                "low",
            ],
            &[("-d", tmp.to_str().unwrap())],
        );
        assert!(ok, "cylinder fixture should compile: {err}");
        assert!(tmp.join("cylinder_fix.stl").exists());
        std::fs::remove_file(tmp.join("cylinder_fix.stl")).ok();
    });
}

#[test]
fn compile_torus_fixture() {
    let src = "define ring\n  units mm\n  torus\n    major 10\n    minor 2\n  end\nend\n";
    with_tmp_polyh("torus_fix", src, |path| {
        let tmp = std::env::temp_dir().join("polyhedra_torus_out");
        std::fs::create_dir_all(&tmp).ok();
        let (ok, _, err) = run_with_outdir(
            &[
                "-c",
                path.to_str().unwrap(),
                "-o",
                "stl",
                "--quality",
                "low",
            ],
            &[("-d", tmp.to_str().unwrap())],
        );
        assert!(ok, "torus fixture should compile: {err}");
        assert!(tmp.join("torus_fix.stl").exists());
        std::fs::remove_file(tmp.join("torus_fix.stl")).ok();
    });
}

#[test]
fn compile_prism_fixture() {
    let src = "define hex\n  units mm\n  prism\n    sides 6\n    flat_to_flat 12\n    height 20\n  end\nend\n";
    with_tmp_polyh("prism_fix", src, |path| {
        let tmp = std::env::temp_dir().join("polyhedra_prism_out");
        std::fs::create_dir_all(&tmp).ok();
        let (ok, _, err) = run_with_outdir(
            &[
                "-c",
                path.to_str().unwrap(),
                "-o",
                "stl",
                "--quality",
                "low",
            ],
            &[("-d", tmp.to_str().unwrap())],
        );
        assert!(ok, "prism fixture should compile: {err}");
        assert!(tmp.join("prism_fix.stl").exists());
        std::fs::remove_file(tmp.join("prism_fix.stl")).ok();
    });
}

#[test]
fn compile_pyramid_fixture() {
    let src = "define pyr\n  units mm\n  pyramid\n    base_width 10\n    base_depth 10\n    height 15\n  end\nend\n";
    with_tmp_polyh("pyramid_fix", src, |path| {
        let tmp = std::env::temp_dir().join("polyhedra_pyramid_out");
        std::fs::create_dir_all(&tmp).ok();
        let (ok, _, err) = run_with_outdir(
            &[
                "-c",
                path.to_str().unwrap(),
                "-o",
                "stl",
                "--quality",
                "low",
            ],
            &[("-d", tmp.to_str().unwrap())],
        );
        assert!(ok, "pyramid fixture should compile: {err}");
        assert!(tmp.join("pyramid_fix.stl").exists());
        std::fs::remove_file(tmp.join("pyramid_fix.stl")).ok();
    });
}

#[test]
fn compile_define_with_fillet() {
    let src = "define rounded\n  units mm\n  cube\n    width 20\n    depth 20\n    height 10\n  end\n  fillet 2\nend\n";
    with_tmp_polyh("fillet_fix", src, |path| {
        let tmp = std::env::temp_dir().join("polyhedra_fillet_out");
        std::fs::create_dir_all(&tmp).ok();
        let (ok, _, err) = run_with_outdir(
            &[
                "-c",
                path.to_str().unwrap(),
                "-o",
                "stl",
                "--quality",
                "low",
            ],
            &[("-d", tmp.to_str().unwrap())],
        );
        assert!(ok, "define with fillet should compile: {err}");
        assert!(tmp.join("fillet_fix.stl").exists());
        std::fs::remove_file(tmp.join("fillet_fix.stl")).ok();
    });
}

#[test]
fn compile_shell_manipulation() {
    let src = "define hollow\n  units mm\n  sphere\n    radius 10\n  end\n  shell 1.5\nend\n";
    with_tmp_polyh("shell_fix", src, |path| {
        let tmp = std::env::temp_dir().join("polyhedra_shell_out");
        std::fs::create_dir_all(&tmp).ok();
        let (ok, _, err) = run_with_outdir(
            &[
                "-c",
                path.to_str().unwrap(),
                "-o",
                "stl",
                "--quality",
                "low",
            ],
            &[("-d", tmp.to_str().unwrap())],
        );
        assert!(ok, "shell manipulation should compile: {err}");
        assert!(tmp.join("shell_fix.stl").exists());
        std::fs::remove_file(tmp.join("shell_fix.stl")).ok();
    });
}

#[test]
fn compile_assemble_union() {
    let src = concat!(
        "define base\n  units mm\n  cube\n    width 20\n    depth 20\n    height 5\n  end\nend\n",
        "define post\n  units mm\n  cylinder\n    radius 3\n    height 15\n  end\nend\n",
        "assemble model\n  place base at origin\n  join post at (0, 0, 5)\nend\n",
    );
    with_tmp_polyh("asm_union", src, |path| {
        let tmp = std::env::temp_dir().join("polyhedra_assemble_out");
        std::fs::create_dir_all(&tmp).ok();
        let (ok, _, err) = run_with_outdir(
            &[
                "-c",
                path.to_str().unwrap(),
                "-o",
                "stl",
                "--quality",
                "low",
            ],
            &[("-d", tmp.to_str().unwrap())],
        );
        assert!(ok, "assemble union should compile: {err}");
        assert!(tmp.join("asm_union.stl").exists());
        std::fs::remove_file(tmp.join("asm_union.stl")).ok();
    });
}

#[test]
fn compile_assemble_cut() {
    let src = concat!(
        "define block\n  units mm\n  cube\n    width 20\n    depth 20\n    height 20\n  end\nend\n",
        "define hole\n  units mm\n  cylinder\n    radius 4\n    height 25\n  end\nend\n",
        "assemble drilled\n  place block at origin\n  cut hole at origin\nend\n",
    );
    with_tmp_polyh("asm_cut", src, |path| {
        let tmp = std::env::temp_dir().join("polyhedra_cut_out");
        std::fs::create_dir_all(&tmp).ok();
        let (ok, _, err) = run_with_outdir(
            &[
                "-c",
                path.to_str().unwrap(),
                "-o",
                "stl",
                "--quality",
                "low",
            ],
            &[("-d", tmp.to_str().unwrap())],
        );
        assert!(ok, "assemble cut should compile: {err}");
        assert!(tmp.join("asm_cut.stl").exists());
        std::fs::remove_file(tmp.join("asm_cut.stl")).ok();
    });
}

#[test]
fn compile_empty_file_exits_nonzero() {
    with_tmp_polyh("empty_fix", "", |path| {
        let (ok, _, err) = run(&["-c", path.to_str().unwrap(), "-o", "stl"]);
        assert!(!ok, "empty .polyh file should fail: {err}");
        assert!(
            err.contains("error") || err.contains("define") || err.contains("assemble"),
            "stderr should explain the error: {err}"
        );
    });
}

// ── Quality levels ──────────────────────────────────────────────────────────────

#[test]
fn quality_medium_produces_more_triangles_than_low() {
    let src = "define quality_sphere\n  units mm\n  sphere\n    radius 10\n  end\nend\n";
    with_tmp_polyh("quality_cmp", src, |path| {
        let tmp_low = std::env::temp_dir().join("polyhedra_quality_low");
        let tmp_med = std::env::temp_dir().join("polyhedra_quality_med");
        std::fs::create_dir_all(&tmp_low).ok();
        std::fs::create_dir_all(&tmp_med).ok();

        let (ok1, _, _) = run_with_outdir(
            &[
                "-c",
                path.to_str().unwrap(),
                "-o",
                "stl",
                "--quality",
                "low",
            ],
            &[("-d", tmp_low.to_str().unwrap())],
        );
        let (ok2, _, _) = run_with_outdir(
            &[
                "-c",
                path.to_str().unwrap(),
                "-o",
                "stl",
                "--quality",
                "medium",
            ],
            &[("-d", tmp_med.to_str().unwrap())],
        );
        assert!(ok1 && ok2, "both quality levels should succeed");

        let read_tri_count = |dir: &Path| -> u32 {
            let data = std::fs::read(dir.join("quality_cmp.stl")).unwrap();
            u32::from_le_bytes([data[80], data[81], data[82], data[83]])
        };

        let low_tris = read_tri_count(&tmp_low);
        let med_tris = read_tri_count(&tmp_med);
        assert!(
            med_tris > low_tris,
            "medium quality ({med_tris}) should produce more triangles than low ({low_tris})"
        );

        std::fs::remove_file(tmp_low.join("quality_cmp.stl")).ok();
        std::fs::remove_file(tmp_med.join("quality_cmp.stl")).ok();
    });
}

// ── GLB format ─────────────────────────────────────────────────────────────────

#[test]
fn compile_hello_produces_valid_glb() {
    let hello = examples_dir().join("hello.polyh");
    let tmp = std::env::temp_dir().join("polyhedra_test_glb");
    std::fs::create_dir_all(&tmp).ok();

    let (ok, _, err) = run_with_outdir(
        &[
            "-c",
            hello.to_str().unwrap(),
            "-o",
            "glb",
            "--quality",
            "low",
        ],
        &[("-d", tmp.to_str().unwrap())],
    );
    assert!(ok, "GLB compile should succeed: {err}");

    let glb_path = tmp.join("hello.glb");
    assert!(glb_path.exists(), "GLB should exist");
    let data = std::fs::read(&glb_path).unwrap();
    assert_eq!(&data[..4], b"glTF", "GLB should start with glTF magic");

    std::fs::remove_file(&glb_path).ok();
}
