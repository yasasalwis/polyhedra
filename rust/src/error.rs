//! Plain-English error types for polyhedra.
//!
//! Every error message is written as a human sentence that a non-technical
//! user can understand and act on. Technical details go in the `source`
//! field, never in the main message.

use thiserror::Error;

/// The top-level error type for all polyhedra operations.
#[derive(Error, Debug)]
pub enum PolyhedraError {
    // ── Parser errors ─────────────────────────────────────────────────────
    #[error("Syntax error on line {line}, column {col}: {message}")]
    ParseError {
        line:    usize,
        col:     usize,
        message: String,
    },

    #[error(
        "Missing unit on line {line}. \
         Try adding a unit like 'mm', 'cm', 'in'. \
         Example: \"radius = 10mm\""
    )]
    MissingUnit { line: usize },

    #[error(
        "Unknown unit '{unit}' on line {line}. \
         Supported units: mm, cm, m, in, ft, degrees, percent."
    )]
    UnknownUnit { unit: String, line: usize },

    #[error(
        "Variable '{name}' is used on line {line} but was never defined. \
         Add \"let {name} = <value>\" before using it."
    )]
    UndefinedVariable { name: String, line: usize },

    #[error(
        "Circular variable reference detected: {cycle}. \
         Variable definitions cannot refer to themselves."
    )]
    CircularReference { cycle: String },

    // ── Import errors ─────────────────────────────────────────────────────
    #[error(
        "File not found: \"{path}\". \
         Check that the file exists and the path is correct."
    )]
    FileNotFound { path: String },

    #[error(
        "Circular import detected: {cycle}. \
         File A cannot use File B if File B also uses File A."
    )]
    CircularImport { cycle: String },

    #[error(
        "File \"{path}\" uses 'assemble as' or 'export:', which are only \
         allowed in main.polyh. Object files can only use 'define' and 'sketch'."
    )]
    AssemblyInObjectFile { path: String },

    #[error(
        "main.polyh uses 'define' or 'sketch', which are not allowed in the \
         main file. Move them to a separate object file and use it with 'use'."
    )]
    DefineInMainFile,

    // ── Geometry errors ───────────────────────────────────────────────────
    #[error("Geometry error: {message}")]
    GeometryError { message: String },

    #[error(
        "Sketch profile '{name}' is not closed. \
         The last point does not connect back to the first point. \
         Gap at approximately ({x:.3}, {y:.3})."
    )]
    OpenProfile { name: String, x: f32, y: f32 },

    #[error(
        "Loft requires all profiles to be closed, but '{name}' is open. \
         Close the profile by connecting the last curve back to the start."
    )]
    LoftOpenProfile { name: String },

    #[error(
        "Mesh resolution is too high. \
         Try a larger refinement value (e.g. 0.01 instead of 0.0001) \
         or reduce model complexity."
    )]
    ResolutionTooHigh,

    #[error(
        "The geometry produced an empty mesh. \
         Check that objects actually overlap or that dimensions are not zero."
    )]
    EmptyMesh,

    // ── Export errors ─────────────────────────────────────────────────────
    #[error("Export error writing to \"{path}\": {message}")]
    ExportError { path: String, message: String },

    #[error(
        "Unknown export format '{format}'. \
         Supported formats: stl, obj, gltf, ply, all."
    )]
    UnknownFormat { format: String },

    // ── I/O errors ────────────────────────────────────────────────────────
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, PolyhedraError>;
