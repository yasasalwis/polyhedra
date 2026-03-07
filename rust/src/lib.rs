//! # polyhedra_core
//!
//! Rust geometry kernel for the polyhedra library.
//!
//! Provides all 3D computation: SDF evaluation, dual-contouring meshing,
//! `.polyh` parsing, and multi-format export. Python bindings (PyO3) are
//! compiled only when the `python` feature is enabled.
//!
//! ## Feature flags
//! - `python` — builds the PyO3 extension module (`_core.so`).
//!              Enabled automatically by maturin. The CLI binary compiles
//!              WITHOUT this flag, so it has no Python dependency.

pub mod error;
pub mod math;
pub mod sdf;
pub mod shape_type;
pub mod units;

// ── Phases 2+ modules (stubs added per phase) ─────────────────────────────
// pub mod parser;
pub mod manipulations;
// pub mod sketch;
// pub mod construction;
// pub mod assembly;
pub mod mesher;
// pub mod export;

// ── Python extension module ────────────────────────────────────────────────
#[cfg(feature = "python")]
use pyo3::prelude::*;

/// PyO3 entry point.
///
/// maturin looks for this function in lib.rs and uses it as the module
/// initialiser for `polyhedra/_core.so`.
///
/// Symbols are added phase by phase. Phase 0 just verifies the module
/// loads successfully in Python.
#[cfg(feature = "python")]
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    // Phase 1+: register Object, Sketch, Assembly, etc.
    Ok(())
}
