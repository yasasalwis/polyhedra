//! Multi-format 3D mesh export.
//!
//! All exporters consume a [`Mesh`] produced by the Dual Contouring mesher and
//! return the serialised bytes, which the caller can write to a file or pipe
//! to a network socket.
//!
//! # Supported formats
//!
//! | Format | Ext   | Encoding         | Notes                              |
//! |--------|-------|------------------|------------------------------------|
//! | STL    | `.stl`| binary LE        | Universal slicer / CAM input       |
//! | OBJ    | `.obj`| UTF-8 text       | Human-readable; includes normals   |
//! | PLY    | `.ply`| binary LE        | MeshLab / CloudCompare friendly    |
//! | GLB    | `.glb`| binary GLTF 2.0  | Web (Three.js, Babylon.js, Godot)  |

pub mod gltf;
pub mod obj;
pub mod ply;
pub mod stl;

use std::path::Path;
use std::str::FromStr;

use glam::Vec3;

use crate::error::{PolyhedraError, Result};
use crate::mesher::Mesh;

// ── ExportFormat ──────────────────────────────────────────────────────────────

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExportFormat {
    Stl,
    Obj,
    Ply,
    Glb,
}

impl ExportFormat {
    /// Canonical file extension (without the leading dot).
    pub fn extension(self) -> &'static str {
        match self {
            Self::Stl => "stl",
            Self::Obj => "obj",
            Self::Ply => "ply",
            Self::Glb => "glb",
        }
    }

    /// Infer format from a file path's extension.  Returns `None` if unknown.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Self::from_str(&ext).ok()
    }
}

impl FromStr for ExportFormat {
    type Err = PolyhedraError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "stl" => Ok(Self::Stl),
            "obj" => Ok(Self::Obj),
            "ply" => Ok(Self::Ply),
            "glb" | "gltf" => Ok(Self::Glb),
            other => Err(PolyhedraError::UnknownFormat {
                format: other.into(),
            }),
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Serialise `mesh` to the requested format and return the bytes.
///
/// Returns [`PolyhedraError::EmptyMesh`] if the mesh has no triangles.
pub fn to_bytes(mesh: &Mesh, fmt: ExportFormat) -> Result<Vec<u8>> {
    if mesh.triangle_count() == 0 {
        return Err(PolyhedraError::EmptyMesh);
    }
    match fmt {
        ExportFormat::Stl => stl::to_bytes(mesh),
        ExportFormat::Obj => obj::to_bytes(mesh),
        ExportFormat::Ply => ply::to_bytes(mesh),
        ExportFormat::Glb => gltf::to_bytes(mesh),
    }
}

/// Write `mesh` to `path`, inferring the format from the file extension.
///
/// Creates (or overwrites) the file at `path`.
pub fn to_file(mesh: &Mesh, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let fmt = ExportFormat::from_path(path).ok_or_else(|| {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("<none>")
            .to_string();
        PolyhedraError::UnknownFormat { format: ext }
    })?;
    let bytes = to_bytes(mesh, fmt)?;
    std::fs::write(path, &bytes)?;
    Ok(())
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Compute per-vertex normals by area-weighted averaging of adjacent face
/// normals.  Vertices with no adjacent faces get a zero normal.
pub(crate) fn compute_vertex_normals(vertices: &[Vec3], triangles: &[[u32; 3]]) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; vertices.len()];
    for &[a, b, c] in triangles {
        let va = vertices[a as usize];
        let vb = vertices[b as usize];
        let vc = vertices[c as usize];
        // Cross product is proportional to face area — naturally area-weights
        // the normal contribution.
        let n = (vb - va).cross(vc - va);
        normals[a as usize] += n;
        normals[b as usize] += n;
        normals[c as usize] += n;
    }
    normals.into_iter().map(|n| n.normalize_or_zero()).collect()
}
