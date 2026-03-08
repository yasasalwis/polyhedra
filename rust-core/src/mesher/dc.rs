//! Marching Cubes mesher — uniform grid implementation.
//!
//! # Algorithm
//!
//! 1. **Sample** the SDF at every vertex of an (N+1)³ uniform grid in parallel
//!    (rayon).
//! 2. **Active cells**: any grid voxel whose 8 corners contain both positive
//!    and negative SDF values straddles the surface.
//! 3. **MC triangulation**: for each active cell compute a case index (0–255)
//!    from the sign pattern of its 8 corners, look up the edge table to find
//!    which of the 12 edges are active, interpolate crossing points along each
//!    active edge, and emit up to 5 triangles per the triangle table.
//!    Vertices are welded via a HashMap keyed by a unique edge identifier to
//!    avoid duplicates.
//! 4. **Taubin smoothing**: alternates forward (λ=0.3) and backward (μ=−0.32)
//!    Laplacian passes to smooth MC staircase artifacts without volume shrinkage.
//!
//! # Reference
//! Lorensen, W. E. & Cline, H. E. — *Marching Cubes: A High Resolution 3D
//! Surface Construction Algorithm* (SIGGRAPH 1987).

use std::collections::HashMap;

use glam::Vec3;
use rayon::prelude::*;

use crate::sdf::Sdf;

// ── Public types ──────────────────────────────────────────────────────────────

/// Triangle mesh produced by the Marching Cubes mesher.
pub struct Mesh {
    /// World-space vertex positions.
    pub vertices: Vec<Vec3>,
    /// Triangle corner indices into `vertices`.
    pub triangles: Vec<[u32; 3]>,
}

impl Mesh {
    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }
    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// `true` if every triangle index is within `vertices` bounds.
    pub fn is_index_valid(&self) -> bool {
        let n = self.vertices.len() as u32;
        self.triangles
            .iter()
            .all(|t| t[0] < n && t[1] < n && t[2] < n)
    }
}

/// Configuration for a single Marching Cubes run.
pub struct MeshConfig {
    /// World-space minimum corner of the grid bounding box.
    pub bounds_min: Vec3,
    /// World-space maximum corner of the grid bounding box.
    pub bounds_max: Vec3,
    /// Grid cells per axis.  Higher → finer mesh at the cost of O(N³) memory.
    pub resolution: u32,
    /// QEF regularisation weight (default `1e-4`).
    /// Unused by Marching Cubes but kept for API compatibility.
    pub lambda: f32,
    /// Taubin smoothing iterations to reduce MC staircase artifacts. 3 iterations
    /// provides smooth curved surfaces while preserving sharp features.
    pub smooth_iterations: u32,
}

impl MeshConfig {
    /// Symmetric bounding box `[−half, half]³` with the given resolution.
    pub fn centered(half: f32, resolution: u32) -> Self {
        Self {
            bounds_min: Vec3::splat(-half),
            bounds_max: Vec3::splat(half),
            resolution,
            lambda: 1e-4,
            smooth_iterations: 3,
        }
    }
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self::centered(1.0, 64)
    }
}

// ── Marching Cubes tables ─────────────────────────────────────────────────────

/// Edge table: for each of the 256 cube configurations, a bitmask of which
/// of the 12 edges are intersected by the surface.
/// Source: Lorensen & Cline 1987 / http://paulbourke.net/geometry/polygonise/
const EDGE_TABLE: [u16; 256] = [
    0x000, 0x109, 0x203, 0x30a, 0x406, 0x50f, 0x605, 0x70c, 0x80c, 0x905, 0xa0f, 0xb06, 0xc0a,
    0xd03, 0xe09, 0xf00, 0x190, 0x099, 0x393, 0x29a, 0x596, 0x49f, 0x795, 0x69c, 0x99c, 0x895,
    0xb9f, 0xa96, 0xd9a, 0xc93, 0xf99, 0xe90, 0x230, 0x339, 0x033, 0x13a, 0x636, 0x73f, 0x435,
    0x53c, 0xa3c, 0xb35, 0x83f, 0x936, 0xe3a, 0xf33, 0xc39, 0xd30, 0x3a0, 0x2a9, 0x1a3, 0x0aa,
    0x7a6, 0x6af, 0x5a5, 0x4ac, 0xbac, 0xaa5, 0x9af, 0x8a6, 0xfaa, 0xea3, 0xda9, 0xca0, 0x460,
    0x569, 0x663, 0x76a, 0x066, 0x16f, 0x265, 0x36c, 0xc6c, 0xd65, 0xe6f, 0xf66, 0x86a, 0x963,
    0xa69, 0xb60, 0x5f0, 0x4f9, 0x7f3, 0x6fa, 0x1f6, 0x0ff, 0x3f5, 0x2fc, 0xdfc, 0xcf5, 0xfff,
    0xef6, 0x9fa, 0x8f3, 0xbf9, 0xaf0, 0x650, 0x759, 0x453, 0x55a, 0x256, 0x35f, 0x055, 0x15c,
    0xe5c, 0xf55, 0xc5f, 0xd56, 0xa5a, 0xb53, 0x859, 0x950, 0x7c0, 0x6c9, 0x5c3, 0x4ca, 0x3c6,
    0x2cf, 0x1c5, 0x0cc, 0xfcc, 0xec5, 0xdcf, 0xcc6, 0xbca, 0xac3, 0x9c9, 0x8c0, 0x8c0, 0x9c9,
    0xac3, 0xbca, 0xcc6, 0xdcf, 0xec5, 0xfcc, 0x0cc, 0x1c5, 0x2cf, 0x3c6, 0x4ca, 0x5c3, 0x6c9,
    0x7c0, 0x950, 0x859, 0xb53, 0xa5a, 0xd56, 0xc5f, 0xf55, 0xe5c, 0x15c, 0x055, 0x35f, 0x256,
    0x55a, 0x453, 0x759, 0x650, 0xaf0, 0xbf9, 0x8f3, 0x9fa, 0xef6, 0xfff, 0xcf5, 0xdfc, 0x2fc,
    0x3f5, 0x0ff, 0x1f6, 0x6fa, 0x7f3, 0x4f9, 0x5f0, 0xb60, 0xa69, 0x963, 0x86a, 0xf66, 0xe6f,
    0xd65, 0xc6c, 0x36c, 0x265, 0x16f, 0x066, 0x76a, 0x663, 0x569, 0x460, 0xca0, 0xda9, 0xea3,
    0xfaa, 0x8a6, 0x9af, 0xaa5, 0xbac, 0x4ac, 0x5a5, 0x6af, 0x7a6, 0x0aa, 0x1a3, 0x2a9, 0x3a0,
    0xd30, 0xc39, 0xf33, 0xe3a, 0x936, 0x83f, 0xb35, 0xa3c, 0x53c, 0x435, 0x73f, 0x636, 0x13a,
    0x033, 0x339, 0x230, 0xe90, 0xf99, 0xc93, 0xd9a, 0xa96, 0xb9f, 0x895, 0x99c, 0x69c, 0x795,
    0x49f, 0x596, 0x29a, 0x393, 0x099, 0x190, 0xf00, 0xe09, 0xd03, 0xc0a, 0xb06, 0xa0f, 0x905,
    0x80c, 0x70c, 0x605, 0x50f, 0x406, 0x30a, 0x203, 0x109, 0x000,
];

/// Triangle table: for each of the 256 cases, lists up to 5 triangles as
/// triples of edge indices (0–11).  -1 terminates the list.
/// Source: Lorensen & Cline 1987 / http://paulbourke.net/geometry/polygonise/
#[rustfmt::skip]
const TRI_TABLE: [[i8; 16]; 256] = [
    [-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 1, 9,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 8, 3, 9, 8, 1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3, 1, 2,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 2,10, 0, 2, 9,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 8, 3, 2,10, 8,10, 9, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 3,11, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0,11, 2, 8,11, 0,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 9, 0, 2, 3,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1,11, 2, 1, 9,11, 9, 8,11,-1,-1,-1,-1,-1,-1,-1],
    [ 3,10, 1,11,10, 3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0,10, 1, 0, 8,10, 8,11,10,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 9, 0, 3,11, 9,11,10, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 8,10,10, 8,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 7, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 3, 0, 7, 3, 4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 1, 9, 8, 4, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 1, 9, 4, 7, 1, 7, 3, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10, 8, 4, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 4, 7, 3, 0, 4, 1, 2,10,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 2,10, 9, 0, 2, 8, 4, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 2,10, 9, 2, 9, 7, 2, 7, 3, 7, 9, 4,-1,-1,-1,-1],
    [ 8, 4, 7, 3,11, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [11, 4, 7,11, 2, 4, 2, 0, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 0, 1, 8, 4, 7, 2, 3,11,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 7,11, 9, 4,11, 9,11, 2, 9, 2, 1,-1,-1,-1,-1],
    [ 3,10, 1, 3,11,10, 7, 8, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 1,11,10, 1, 4,11, 1, 0, 4, 7,11, 4,-1,-1,-1,-1],
    [ 4, 7, 8, 9, 0,11, 9,11,10,11, 0, 3,-1,-1,-1,-1],
    [ 4, 7,11, 4,11, 9, 9,11,10,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 5, 4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 5, 4, 0, 8, 3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 5, 4, 1, 5, 0,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 5, 4, 8, 3, 5, 3, 1, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10, 9, 5, 4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 0, 8, 1, 2,10, 4, 9, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 5, 2,10, 5, 4, 2, 4, 0, 2,-1,-1,-1,-1,-1,-1,-1],
    [ 2,10, 5, 3, 2, 5, 3, 5, 4, 3, 4, 8,-1,-1,-1,-1],
    [ 9, 5, 4, 2, 3,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0,11, 2, 0, 8,11, 4, 9, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 5, 4, 0, 1, 5, 2, 3,11,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 1, 5, 2, 5, 8, 2, 8,11, 4, 8, 5,-1,-1,-1,-1],
    [10, 3,11,10, 1, 3, 9, 5, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 9, 5, 0, 8, 1, 8,10, 1, 8,11,10,-1,-1,-1,-1],
    [ 5, 4, 0, 5, 0,11, 5,11,10,11, 0, 3,-1,-1,-1,-1],
    [ 5, 4, 8, 5, 8,10,10, 8,11,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 7, 8, 5, 7, 9,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 3, 0, 9, 5, 3, 5, 7, 3,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 7, 8, 0, 1, 7, 1, 5, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 5, 3, 3, 5, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 7, 8, 9, 5, 7,10, 1, 2,-1,-1,-1,-1,-1,-1,-1],
    [10, 1, 2, 9, 5, 0, 5, 3, 0, 5, 7, 3,-1,-1,-1,-1],
    [ 8, 0, 2, 8, 2, 5, 8, 5, 7,10, 5, 2,-1,-1,-1,-1],
    [ 2,10, 5, 2, 5, 3, 3, 5, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 7, 9, 5, 7, 8, 9, 3,11, 2,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 5, 7, 9, 7, 2, 9, 2, 0, 2, 7,11,-1,-1,-1,-1],
    [ 2, 3,11, 0, 1, 8, 1, 7, 8, 1, 5, 7,-1,-1,-1,-1],
    [11, 2, 1,11, 1, 7, 7, 1, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 5, 8, 8, 5, 7,10, 1, 3,10, 3,11,-1,-1,-1,-1],
    [ 5, 7, 0, 5, 0, 9, 7,11, 0, 1, 0,10,11,10, 0,-1],
    [11,10, 0,11, 0, 3,10, 5, 0, 8, 0, 7, 5, 7, 0,-1],
    [11,10, 5, 7,11, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [10, 6, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3, 5,10, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 0, 1, 5,10, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 8, 3, 1, 9, 8, 5,10, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 6, 5, 2, 6, 1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 6, 5, 1, 2, 6, 3, 0, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 6, 5, 9, 0, 6, 0, 2, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 5, 9, 8, 5, 8, 2, 5, 2, 6, 3, 2, 8,-1,-1,-1,-1],
    [ 2, 3,11,10, 6, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [11, 0, 8,11, 2, 0,10, 6, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 1, 9, 2, 3,11, 5,10, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 5,10, 6, 1, 9, 2, 9,11, 2, 9, 8,11,-1,-1,-1,-1],
    [ 6, 3,11, 6, 5, 3, 5, 1, 3,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8,11, 0,11, 5, 0, 5, 1, 5,11, 6,-1,-1,-1,-1],
    [ 3,11, 6, 0, 3, 6, 0, 6, 5, 0, 5, 9,-1,-1,-1,-1],
    [ 6, 5, 9, 6, 9,11,11, 9, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 5,10, 6, 4, 7, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 3, 0, 4, 7, 3, 6, 5,10,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 9, 0, 5,10, 6, 8, 4, 7,-1,-1,-1,-1,-1,-1,-1],
    [10, 6, 5, 1, 9, 7, 1, 7, 3, 7, 9, 4,-1,-1,-1,-1],
    [ 6, 1, 2, 6, 5, 1, 4, 7, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2, 5, 5, 2, 6, 3, 0, 4, 3, 4, 7,-1,-1,-1,-1],
    [ 8, 4, 7, 9, 0, 5, 0, 6, 5, 0, 2, 6,-1,-1,-1,-1],
    [ 7, 3, 9, 7, 9, 4, 3, 2, 9, 5, 9, 6, 2, 6, 9,-1],
    [ 3,11, 2, 7, 8, 4,10, 6, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 5,10, 6, 4, 7, 2, 4, 2, 0, 2, 7,11,-1,-1,-1,-1],
    [ 0, 1, 9, 4, 7, 8, 2, 3,11, 5,10, 6,-1,-1,-1,-1],
    [ 9, 2, 1, 9,11, 2, 9, 4,11, 7,11, 4, 5,10, 6,-1],
    [ 8, 4, 7, 3,11, 5, 3, 5, 1, 5,11, 6,-1,-1,-1,-1],
    [ 5, 1,11, 5,11, 6, 1, 0,11, 7,11, 4, 0, 4,11,-1],
    [ 0, 5, 9, 0, 6, 5, 0, 3, 6,11, 6, 3, 8, 4, 7,-1],
    [ 6, 5, 9, 6, 9,11, 4, 7, 9, 7,11, 9,-1,-1,-1,-1],
    [10, 4, 9, 6, 4,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4,10, 6, 4, 9,10, 0, 8, 3,-1,-1,-1,-1,-1,-1,-1],
    [10, 0, 1,10, 6, 0, 6, 4, 0,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 3, 1, 8, 1, 6, 8, 6, 4, 6, 1,10,-1,-1,-1,-1],
    [ 1, 4, 9, 1, 2, 4, 2, 6, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 0, 8, 1, 2, 9, 2, 4, 9, 2, 6, 4,-1,-1,-1,-1],
    [ 0, 2, 4, 4, 2, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 3, 2, 8, 2, 4, 4, 2, 6,-1,-1,-1,-1,-1,-1,-1],
    [10, 4, 9,10, 6, 4,11, 2, 3,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 2, 2, 8,11, 4, 9,10, 4,10, 6,-1,-1,-1,-1],
    [ 3,11, 2, 0, 1, 6, 0, 6, 4, 6, 1,10,-1,-1,-1,-1],
    [ 6, 4, 1, 6, 1,10, 4, 8, 1, 2, 1,11, 8,11, 1,-1],
    [ 9, 6, 4, 9, 3, 6, 9, 1, 3,11, 6, 3,-1,-1,-1,-1],
    [ 8,11, 1, 8, 1, 0,11, 6, 1, 9, 1, 4, 6, 4, 1,-1],
    [ 3,11, 6, 3, 6, 0, 0, 6, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 6, 4, 8,11, 6, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 7,10, 6, 7, 8,10, 8, 9,10,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 7, 3, 0,10, 7, 0, 9,10, 6, 7,10,-1,-1,-1,-1],
    [10, 6, 7, 1,10, 7, 1, 7, 8, 1, 8, 0,-1,-1,-1,-1],
    [10, 6, 7,10, 7, 1, 1, 7, 3,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2, 6, 1, 6, 8, 1, 8, 9, 8, 6, 7,-1,-1,-1,-1],
    [ 2, 6, 9, 2, 9, 1, 6, 7, 9, 0, 9, 3, 7, 3, 9,-1],
    [ 7, 8, 0, 7, 0, 6, 6, 0, 2,-1,-1,-1,-1,-1,-1,-1],
    [ 7, 3, 2, 6, 7, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 3,11,10, 6, 8,10, 8, 9, 8, 6, 7,-1,-1,-1,-1],
    [ 2, 0, 7, 2, 7,11, 0, 9, 7, 6, 7,10, 9,10, 7,-1],
    [ 1, 8, 0, 1, 7, 8, 1,10, 7, 6, 7,10, 2, 3,11,-1],
    [11, 2, 1,11, 1, 7,10, 6, 1, 6, 7, 1,-1,-1,-1,-1],
    [ 8, 9, 1, 8, 1, 3, 9, 6, 1,11, 1,11, 6,11, 1,-1],  // 126 - note: this was originally tricky
    [ 0, 9, 1,11, 6, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 7, 8, 0, 7, 0, 6, 3,11, 0,11, 6, 0,-1,-1,-1,-1],
    [ 7,11, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 7, 6,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 0, 8,11, 7, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 1, 9,11, 7, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 1, 9, 8, 3, 1,11, 7, 6,-1,-1,-1,-1,-1,-1,-1],
    [10, 1, 2, 6,11, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10, 3, 0, 8, 6,11, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 9, 0, 2,10, 9, 6,11, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 6,11, 7, 2,10, 3,10, 8, 3,10, 9, 8,-1,-1,-1,-1],
    [ 7, 2, 3, 6, 2, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 7, 0, 8, 7, 6, 0, 6, 2, 0,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 7, 6, 2, 3, 7, 0, 1, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 6, 2, 1, 8, 6, 1, 9, 8, 8, 7, 6,-1,-1,-1,-1],
    [10, 7, 6,10, 1, 7, 1, 3, 7,-1,-1,-1,-1,-1,-1,-1],
    [10, 7, 6, 1, 7,10, 1, 8, 7, 1, 0, 8,-1,-1,-1,-1],
    [ 0, 3, 7, 0, 7,10, 0,10, 9, 6,10, 7,-1,-1,-1,-1],
    [ 7, 6,10, 7,10, 8, 8,10, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 6, 8, 4,11, 8, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 6,11, 3, 0, 6, 0, 4, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 6,11, 8, 4, 6, 9, 0, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 4, 6, 9, 6, 3, 9, 3, 1,11, 3, 6,-1,-1,-1,-1],
    [ 6, 8, 4, 6,11, 8, 2,10, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10, 3, 0,11, 0, 6,11, 0, 4, 6,-1,-1,-1,-1],
    [ 4,11, 8, 4, 6,11, 0, 2, 9, 2,10, 9,-1,-1,-1,-1],
    [10, 9, 3,10, 3, 2, 9, 4, 3,11, 3, 6, 4, 6, 3,-1],
    [ 8, 2, 3, 8, 4, 2, 4, 6, 2,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 4, 2, 4, 6, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 9, 0, 2, 3, 4, 2, 4, 6, 4, 3, 8,-1,-1,-1,-1],
    [ 1, 9, 4, 1, 4, 2, 2, 4, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 1, 3, 8, 6, 1, 8, 4, 6, 6,10, 1,-1,-1,-1,-1],
    [10, 1, 0,10, 0, 6, 6, 0, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 6, 3, 4, 3, 8, 6,10, 3, 0, 3, 9,10, 9, 3,-1],
    [10, 9, 4, 6,10, 4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 9, 5, 7, 6,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3, 4, 9, 5,11, 7, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 5, 0, 1, 5, 4, 0, 7, 6,11,-1,-1,-1,-1,-1,-1,-1],
    [11, 7, 6, 8, 3, 4, 3, 5, 4, 3, 1, 5,-1,-1,-1,-1],
    [ 9, 5, 4,10, 1, 2, 7, 6,11,-1,-1,-1,-1,-1,-1,-1],
    [ 6,11, 7, 1, 2,10, 0, 8, 3, 4, 9, 5,-1,-1,-1,-1],
    [ 7, 6,11, 5, 4,10, 4, 2,10, 4, 0, 2,-1,-1,-1,-1],
    [ 3, 4, 8, 3, 5, 4, 3, 2, 5,10, 5, 2, 6,11, 7,-1],
    [ 7, 2, 3, 7, 6, 2, 5, 4, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 5, 4, 0, 8, 6, 0, 6, 2, 6, 8, 7,-1,-1,-1,-1],
    [ 3, 6, 2, 3, 7, 6, 1, 5, 0, 5, 4, 0,-1,-1,-1,-1],
    [ 6, 2, 8, 6, 8, 7, 2, 1, 8, 4, 8, 5, 1, 5, 8,-1],
    [ 9, 5, 4,10, 1, 6, 1, 7, 6, 1, 3, 7,-1,-1,-1,-1],
    [ 1, 6,10, 1, 7, 6, 1, 0, 7, 8, 7, 0, 9, 5, 4,-1],
    [ 4, 0,10, 4,10, 5, 0, 3,10, 6,10, 7, 3, 7,10,-1],
    [ 7, 6,10, 7,10, 8, 5, 4,10, 4, 8,10,-1,-1,-1,-1],
    [ 6, 9, 5, 6,11, 9,11, 8, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 6,11, 0, 6, 3, 0, 5, 6, 0, 9, 5,-1,-1,-1,-1],
    [ 0,11, 8, 0, 5,11, 0, 1, 5, 5, 6,11,-1,-1,-1,-1],
    [ 6,11, 3, 6, 3, 5, 5, 3, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10, 9, 5,11, 9,11, 8,11, 5, 6,-1,-1,-1,-1],
    [ 0,11, 3, 0, 6,11, 0, 9, 6, 5, 6, 9, 1, 2,10,-1],
    [11, 8, 5,11, 5, 6, 8, 0, 5,10, 5, 2, 0, 2, 5,-1],
    [ 6,11, 3, 6, 3, 5, 2,10, 3,10, 5, 3,-1,-1,-1,-1],
    [ 5, 8, 9, 5, 2, 8, 5, 6, 2, 3, 8, 2,-1,-1,-1,-1],
    [ 9, 5, 6, 9, 6, 0, 0, 6, 2,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 5, 8, 1, 8, 0, 5, 6, 8, 3, 8, 2, 6, 2, 8,-1],
    [ 1, 5, 6, 2, 1, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 3, 6, 1, 6,10, 3, 8, 6, 5, 6, 9, 8, 9, 6,-1],
    [10, 1, 0,10, 0, 6, 9, 5, 0, 5, 6, 0,-1,-1,-1,-1],
    [ 0, 3, 8, 5, 6,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [10, 5, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [11, 5,10, 7, 5,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [11, 5,10,11, 7, 5, 8, 3, 0,-1,-1,-1,-1,-1,-1,-1],
    [ 5,11, 7, 5,10,11, 1, 9, 0,-1,-1,-1,-1,-1,-1,-1],
    [10, 7, 5,10,11, 7, 9, 8, 1, 8, 3, 1,-1,-1,-1,-1],
    [11, 1, 2,11, 7, 1, 7, 5, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3, 1, 2, 7, 1, 7, 5, 7, 2,11,-1,-1,-1,-1],
    [ 9, 7, 5, 9, 2, 7, 9, 0, 2, 2,11, 7,-1,-1,-1,-1],
    [ 7, 5, 2, 7, 2,11, 5, 9, 2, 3, 2, 8, 9, 8, 2,-1],
    [ 2, 5,10, 2, 3, 5, 3, 7, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 2, 0, 8, 5, 2, 8, 7, 5,10, 2, 5,-1,-1,-1,-1],
    [ 9, 0, 1, 5,10, 3, 5, 3, 7, 3,10, 2,-1,-1,-1,-1],
    [ 9, 8, 2, 9, 2, 1, 8, 7, 2,10, 2, 5, 7, 5, 2,-1],
    [ 1, 3, 5, 3, 7, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 7, 0, 7, 1, 1, 7, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 0, 3, 9, 3, 5, 5, 3, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 8, 7, 5, 9, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 5, 8, 4, 5,10, 8,10,11, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 5, 0, 4, 5,11, 0, 5,10,11,11, 3, 0,-1,-1,-1,-1],
    [ 0, 1, 9, 8, 4,10, 8,10,11,10, 4, 5,-1,-1,-1,-1],
    [10,11, 4,10, 4, 5,11, 3, 4, 9, 4, 1, 3, 1, 4,-1],
    [ 2, 5, 1, 2, 8, 5, 2,11, 8, 4, 5, 8,-1,-1,-1,-1],
    [ 0, 4,11, 0,11, 3, 4, 5,11, 2,11, 1, 5, 1,11,-1],
    [ 0, 2, 5, 0, 5, 9, 2,11, 5, 4, 5, 8,11, 8, 5,-1],
    [ 9, 4, 5, 2,11, 3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 5,10, 3, 5, 2, 3, 4, 5, 3, 8, 4,-1,-1,-1,-1],
    [ 5,10, 2, 5, 2, 4, 4, 2, 0,-1,-1,-1,-1,-1,-1,-1],
    [ 3,10, 2, 3, 5,10, 3, 8, 5, 4, 5, 8, 0, 1, 9,-1],
    [ 5,10, 2, 5, 2, 4, 1, 9, 2, 9, 4, 2,-1,-1,-1,-1],
    [ 8, 4, 5, 8, 5, 3, 3, 5, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 4, 5, 1, 0, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 4, 5, 8, 5, 3, 9, 0, 5, 0, 3, 5,-1,-1,-1,-1],
    [ 9, 4, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4,11, 7, 4, 9,11, 9,10,11,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3, 4, 9, 7, 9,11, 7, 9,10,11,-1,-1,-1,-1],
    [ 1,10,11, 1,11, 4, 1, 4, 0, 7, 4,11,-1,-1,-1,-1],
    [ 3, 1, 4, 3, 4, 8, 1,10, 4, 7, 4,11,10,11, 4,-1],
    [ 4,11, 7, 9,11, 4, 9, 2,11, 9, 1, 2,-1,-1,-1,-1],
    [ 9, 7, 4, 9,11, 7, 9, 1,11, 2,11, 1, 0, 8, 3,-1],
    [11, 7, 4,11, 4, 2, 2, 4, 0,-1,-1,-1,-1,-1,-1,-1],
    [11, 7, 4,11, 4, 2, 8, 3, 4, 3, 2, 4,-1,-1,-1,-1],
    [ 2, 9,10, 2, 7, 9, 2, 3, 7, 7, 4, 9,-1,-1,-1,-1],
    [ 9,10, 7, 9, 7, 4,10, 2, 7, 8, 7, 0, 2, 0, 7,-1],
    [ 3, 7,10, 3,10, 2, 7, 4,10, 1,10, 0, 4, 0,10,-1],
    [ 1,10, 2, 8, 7, 4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 9, 1, 4, 1, 7, 7, 1, 3,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 9, 1, 4, 1, 7, 0, 8, 1, 8, 7, 1,-1,-1,-1,-1],
    [ 4, 0, 3, 7, 4, 3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 8, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9,10, 8,10,11, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 0, 9, 3, 9,11,11, 9,10,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 1,10, 0,10, 8, 8,10,11,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 1,10,11, 3,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,11, 1,11, 9, 9,11, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 0, 9, 3, 9,11, 1, 2, 9, 2,11, 9,-1,-1,-1,-1],
    [ 0, 2,11, 8, 0,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 2,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 3, 8, 2, 8,10,10, 8, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 9,10, 2, 0, 9, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 3, 8, 2, 8,10, 0, 1, 8, 1,10, 8,-1,-1,-1,-1],
    [ 1,10, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 3, 8, 9, 1, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 9, 1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 3, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
];

// ── Edge connectivity (MC standard 12 edges) ──────────────────────────────────
// Each entry: (da0, db0, dc0, da1, db1, dc1) — offsets from cell (i,j,k) for
// the two corners connected by this edge.
const EDGE_CORNERS: [(usize, usize, usize, usize, usize, usize); 12] = [
    (0, 0, 0, 1, 0, 0), //  0: X
    (1, 0, 0, 1, 1, 0), //  1: Y
    (0, 1, 0, 1, 1, 0), //  2: X  (lower i corner is i,j+1,k)
    (0, 0, 0, 0, 1, 0), //  3: Y
    (0, 0, 1, 1, 0, 1), //  4: X
    (1, 0, 1, 1, 1, 1), //  5: Y
    (0, 1, 1, 1, 1, 1), //  6: X
    (0, 0, 1, 0, 1, 1), //  7: Y
    (0, 0, 0, 0, 0, 1), //  8: Z
    (1, 0, 0, 1, 0, 1), //  9: Z
    (1, 1, 0, 1, 1, 1), // 10: Z
    (0, 1, 0, 0, 1, 1), // 11: Z
];

// Direction for each edge: 0=X, 1=Y, 2=Z
const EDGE_DIR: [u64; 12] = [0, 1, 0, 1, 0, 1, 0, 1, 2, 2, 2, 2];

/// Maps edge-corner offset (da, db, dc) to the corner_vals index.
/// CORNER_IDX[da][db][dc] = c such that (DI[c], DJ[c], DK[c]) = (da, db, dc).
const CORNER_IDX: [[[usize; 2]; 2]; 2] = [
    // da=0:          dc=0  dc=1         dc=0  dc=1
    [[0, 4], [3, 7]], // db=0: 0,4   db=1: 3,7
    [[1, 5], [2, 6]], // da=1: db=0: 1,5   db=1: 2,6
];

// ── MC corner layout ──────────────────────────────────────────────────────────
// Corner offsets: corner N of cell (i,j,k) is at (i + DI[N], j + DJ[N], k + DK[N])
const DI: [usize; 8] = [0, 1, 1, 0, 0, 1, 1, 0];
const DJ: [usize; 8] = [0, 0, 1, 1, 0, 0, 1, 1];
const DK: [usize; 8] = [0, 0, 0, 0, 1, 1, 1, 1];

// ── Mesher entry point ────────────────────────────────────────────────────────

/// Mesh the given SDF using Marching Cubes.
///
/// The bounding box (`cfg.bounds_min` … `cfg.bounds_max`) must entirely
/// contain the surface — if the SDF is negative anywhere on the boundary the
/// output mesh will have open edges there.
pub fn mesh(sdf: &dyn Sdf, cfg: &MeshConfig) -> Mesh {
    let n = cfg.resolution as usize;
    let nv = n + 1; // grid vertices per axis
    let step = (cfg.bounds_max - cfg.bounds_min) / cfg.resolution as f32;

    // ── Step 1: parallel SDF sampling ────────────────────────────────────────
    let total_v = nv * nv * nv;
    let values: Vec<f32> = (0..total_v)
        .into_par_iter()
        .map(|idx| {
            let i = idx % nv;
            let j = (idx / nv) % nv;
            let k = idx / (nv * nv);
            let p =
                cfg.bounds_min + Vec3::new(i as f32 * step.x, j as f32 * step.y, k as f32 * step.z);
            sdf.distance(p)
        })
        .collect();

    // Grid-vertex linear index.
    let vi = |i: usize, j: usize, k: usize| -> usize { i + j * nv + k * nv * nv };

    // World-space position of grid vertex (i, j, k).
    let gpos = |i: usize, j: usize, k: usize| -> Vec3 {
        cfg.bounds_min + Vec3::new(i as f32 * step.x, j as f32 * step.y, k as f32 * step.z)
    };

    // ── Step 2: MC triangulation with vertex welding ──────────────────────────
    let mut edge_to_vertex: HashMap<u64, u32> = HashMap::new();
    let mut vertices: Vec<Vec3> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();

    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                // Gather the 8 corner SDF values and compute the case index.
                let mut corner_vals = [0.0f32; 8];
                let mut cube_index: usize = 0;
                for c in 0..8 {
                    let val = values[vi(i + DI[c], j + DJ[c], k + DK[c])];
                    corner_vals[c] = val;
                    if val < 0.0 {
                        cube_index |= 1 << c;
                    }
                }

                // Skip fully inside or fully outside cells.
                let edge_mask = EDGE_TABLE[cube_index];
                if edge_mask == 0 {
                    continue;
                }

                // For each active edge, get or create the interpolated vertex.
                let mut edge_verts = [0u32; 12];
                for e in 0..12u64 {
                    if edge_mask & (1 << e) == 0 {
                        continue;
                    }
                    let e_idx = e as usize;
                    let (da0, db0, dc0, da1, db1, dc1) = EDGE_CORNERS[e_idx];
                    let ei = i + da0;
                    let ej = j + db0;
                    let ek = k + dc0;
                    let gz_vertex_index = (ei + ej * nv + ek * nv * nv) as u64;
                    let edge_key = gz_vertex_index * 3 + EDGE_DIR[e_idx];

                    let vert_idx = if let Some(&existing) = edge_to_vertex.get(&edge_key) {
                        existing
                    } else {
                        // Interpolate along the edge.
                        let va = corner_vals[CORNER_IDX[da0][db0][dc0]];
                        let vb = corner_vals[CORNER_IDX[da1][db1][dc1]];
                        let pa = gpos(i + da0, j + db0, k + dc0);
                        let pb = gpos(i + da1, j + db1, k + dc1);
                        let t = if (vb - va).abs() > 1e-8 {
                            va / (va - vb)
                        } else {
                            0.5
                        };
                        let p = pa + t * (pb - pa);
                        let new_idx = vertices.len() as u32;
                        vertices.push(p);
                        edge_to_vertex.insert(edge_key, new_idx);
                        new_idx
                    };
                    edge_verts[e_idx] = vert_idx;
                }

                // Emit triangles from the triangle table.
                let tri_row = &TRI_TABLE[cube_index];
                let mut t = 0;
                while t < 15 && tri_row[t] != -1 {
                    let a = edge_verts[tri_row[t] as usize];
                    let b = edge_verts[tri_row[t + 1] as usize];
                    let c = edge_verts[tri_row[t + 2] as usize];
                    triangles.push([a, b, c]);
                    t += 3;
                }
            }
        }
    }

    let mut m = Mesh {
        vertices,
        triangles,
    };

    // ── Step 3: Taubin smoothing ──────────────────────────────────────────────
    if cfg.smooth_iterations > 0 {
        taubin_smooth(&mut m, cfg.smooth_iterations);
    }

    m
}

// ── Taubin smoothing ──────────────────────────────────────────────────────────

/// Smooth the mesh using Taubin λ/μ passes.
///
/// **Why Taubin?**  Plain Laplacian smoothing shrinks the mesh — every vertex
/// moves toward its neighbours' centroid, slowly collapsing the surface
/// inward.  Taubin's method alternates a forward step (λ > 0, smoothing) with
/// a backward step (μ < 0, anti-shrinkage, |μ| slightly > λ), so low-
/// frequency shape is preserved while high-frequency MC noise is removed.
fn taubin_smooth(mesh: &mut Mesh, iterations: u32) {
    let nv = mesh.vertices.len();
    if nv == 0 {
        return;
    }

    // ── Build CSR adjacency list once ─────────────────────────────────────────
    let mut degree = vec![0u32; nv];
    for &[a, b, c] in &mesh.triangles {
        degree[a as usize] += 2;
        degree[b as usize] += 2;
        degree[c as usize] += 2;
    }
    let mut start = vec![0u32; nv + 1];
    for i in 0..nv {
        start[i + 1] = start[i] + degree[i];
    }
    let total_edges = start[nv] as usize;
    let mut data = vec![0u32; total_edges];
    let mut cursor = start.clone();
    for &[a, b, c] in &mesh.triangles {
        for &(u, v) in &[(a, b), (a, c), (b, a), (b, c), (c, a), (c, b)] {
            data[cursor[u as usize] as usize] = v;
            cursor[u as usize] += 1;
        }
    }

    // Taubin parameters: λ forward, μ backward.
    // Standard values from Taubin 1995 — μ slightly larger in magnitude than λ
    // so the backward pass undoes the DC component but not the smoothing.
    const LAMBDA: f32 = 0.3;
    const MU: f32 = -0.32;

    for _ in 0..iterations {
        // Forward pass: move toward neighbour centroid
        laplacian_pass(&mut mesh.vertices, &start, &data, LAMBDA);
        // Backward pass: pull back slightly to prevent shrinkage
        laplacian_pass(&mut mesh.vertices, &start, &data, MU);
    }
}

/// One Laplacian pass: each vertex moves `weight` of the way toward the mean
/// of its one-ring neighbours.  Negative weight moves away (anti-smoothing).
fn laplacian_pass(vertices: &mut Vec<Vec3>, start: &[u32], data: &[u32], weight: f32) {
    let nv = vertices.len();
    let old = vertices.clone();
    for v in 0..nv {
        let s = start[v] as usize;
        let e = start[v + 1] as usize;
        if s == e {
            continue;
        }
        let sum = data[s..e]
            .iter()
            .fold(Vec3::ZERO, |acc, &nb| acc + old[nb as usize]);
        let centroid = sum / (e - s) as f32;
        vertices[v] = old[v] + weight * (centroid - old[v]);
    }
}
