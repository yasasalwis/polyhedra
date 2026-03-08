//! Dual Contouring mesher — uniform grid implementation.
//!
//! # Algorithm
//!
//! 1. **Sample** the SDF at every vertex of an (N+1)³ uniform grid in parallel
//!    (rayon).
//! 2. **Active cells**: any grid voxel whose 8 corners contain both positive
//!    and negative SDF values straddles the surface.
//! 3. **QEF vertex**: for each active cell collect all 12 edges that cross zero,
//!    find the crossing point by linear interpolation, sample the SDF normal,
//!    and place the dual vertex by minimising the Quadratic Error Function.
//!    The vertex is clamped to the cell's bounding box.
//! 4. **Dual quads**: for every grid edge that crosses zero, the four cells that
//!    share that edge contribute their dual vertices as a quad (split into two
//!    triangles).  Winding is chosen so that the outward face normal agrees
//!    with the direction the SDF increases.
//!
//! # Reference
//! Ju, Losasso, Schaefer, Warren — *Dual Contouring of Hermite Data* (SIGGRAPH
//! 2002).

use glam::Vec3;
use rayon::prelude::*;

use crate::sdf::Sdf;

use super::qef::Qef;

// ── Public types ──────────────────────────────────────────────────────────────

/// Triangle mesh produced by the Dual Contouring mesher.
pub struct Mesh {
    /// World-space vertex positions.
    pub vertices: Vec<Vec3>,
    /// Triangle corner indices into `vertices`.
    pub triangles: Vec<[u32; 3]>,
}

impl Mesh {
    /// Number of triangles.
    pub fn triangle_count(&self) -> usize { self.triangles.len() }
    /// Number of vertices.
    pub fn vertex_count(&self) -> usize { self.vertices.len() }

    /// `true` if every triangle index is within `vertices` bounds.
    pub fn is_index_valid(&self) -> bool {
        let n = self.vertices.len() as u32;
        self.triangles.iter().all(|t| t[0] < n && t[1] < n && t[2] < n)
    }
}

/// Configuration for a single Dual Contouring run.
pub struct MeshConfig {
    /// World-space minimum corner of the grid bounding box.
    pub bounds_min: Vec3,
    /// World-space maximum corner of the grid bounding box.
    pub bounds_max: Vec3,
    /// Grid cells per axis.  Higher → finer mesh at the cost of O(N³) memory.
    pub resolution: u32,
    /// QEF regularisation weight (default `1e-3`).
    pub lambda: f32,
}

impl MeshConfig {
    /// Symmetric bounding box `[−half, half]³` with the given resolution.
    pub fn centered(half: f32, resolution: u32) -> Self {
        Self {
            bounds_min: Vec3::splat(-half),
            bounds_max: Vec3::splat(half),
            resolution,
            lambda: 1e-3,
        }
    }
}

impl Default for MeshConfig {
    fn default() -> Self { Self::centered(1.0, 32) }
}

// ── Mesher entry point ────────────────────────────────────────────────────────

/// Mesh the given SDF using Dual Contouring.
///
/// The bounding box (`cfg.bounds_min` … `cfg.bounds_max`) must entirely
/// contain the surface — if the SDF is negative anywhere on the boundary the
/// output mesh will have open edges there.
pub fn mesh(sdf: &dyn Sdf, cfg: &MeshConfig) -> Mesh {
    let n  = cfg.resolution as usize;
    let nv = n + 1; // grid vertices per axis
    let step = (cfg.bounds_max - cfg.bounds_min) / cfg.resolution as f32;
    let lam  = cfg.lambda as f64;

    // ── Step 1: parallel SDF sampling ────────────────────────────────────────
    let total_v = nv * nv * nv;
    let values: Vec<f32> = (0..total_v)
        .into_par_iter()
        .map(|idx| {
            let i = idx % nv;
            let j = (idx / nv) % nv;
            let k = idx / (nv * nv);
            let p = cfg.bounds_min
                + Vec3::new(i as f32 * step.x, j as f32 * step.y, k as f32 * step.z);
            sdf.distance(p)
        })
        .collect();

    // Grid-vertex linear index.
    let vi = |i: usize, j: usize, k: usize| i + j * nv + k * nv * nv;

    // World-space position of grid vertex (i, j, k).
    let gpos = |i: usize, j: usize, k: usize| -> Vec3 {
        cfg.bounds_min
            + Vec3::new(i as f32 * step.x, j as f32 * step.y, k as f32 * step.z)
    };

    // ── Step 2: active cells → QEF vertices ──────────────────────────────────
    // The 12 edges of a unit cube as pairs of (dx, dy, dz) corner offsets.
    // Corner index layout: dx + dy*2 + dz*4.
    const EDGES: [(usize, usize, usize, usize, usize, usize); 12] = [
        // X-parallel
        (0,0,0, 1,0,0), (0,1,0, 1,1,0), (0,0,1, 1,0,1), (0,1,1, 1,1,1),
        // Y-parallel
        (0,0,0, 0,1,0), (1,0,0, 1,1,0), (0,0,1, 0,1,1), (1,0,1, 1,1,1),
        // Z-parallel
        (0,0,0, 0,0,1), (1,0,0, 1,0,1), (0,1,0, 0,1,1), (1,1,0, 1,1,1),
    ];

    // cell_vertex[i + j*n + k*n*n] = Some(vertex_index) for active cells.
    let mut cell_vertex: Vec<Option<u32>> = vec![None; n * n * n];
    let mut vertices: Vec<Vec3>           = Vec::new();

    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                // Eight corner values: c[dx + dy*2 + dz*4]
                let c = [
                    values[vi(i,   j,   k)],   // 0 = (0,0,0)
                    values[vi(i+1, j,   k)],   // 1 = (1,0,0)
                    values[vi(i,   j+1, k)],   // 2 = (0,1,0)
                    values[vi(i+1, j+1, k)],   // 3 = (1,1,0)
                    values[vi(i,   j,   k+1)], // 4 = (0,0,1)
                    values[vi(i+1, j,   k+1)], // 5 = (1,0,1)
                    values[vi(i,   j+1, k+1)], // 6 = (0,1,1)
                    values[vi(i+1, j+1, k+1)], // 7 = (1,1,1)
                ];

                let has_neg = c.iter().any(|&v| v < 0.0);
                let has_pos = c.iter().any(|&v| v >= 0.0);
                if !has_neg || !has_pos { continue; }

                // Accumulate QEF from all sign-changing edges.
                let mut qef = Qef::new();
                for &(ax, ay, az, bx, by, bz) in &EDGES {
                    let va = c[ax + ay * 2 + az * 4];
                    let vb = c[bx + by * 2 + bz * 4];
                    if (va < 0.0) == (vb < 0.0) { continue; }

                    // Linear-interpolation crossing point.
                    let t = va / (va - vb);
                    let pa = gpos(i + ax, j + ay, k + az);
                    let pb = gpos(i + bx, j + by, k + bz);
                    let p  = pa + t * (pb - pa);
                    qef.add(p, sdf.normal(p));
                }

                let cell_min    = gpos(i, j, k);
                let cell_max    = gpos(i + 1, j + 1, k + 1);
                let cell_center = (cell_min + cell_max) * 0.5;
                let v = qef.solve(cell_center, lam).clamp(cell_min, cell_max);

                let idx = vertices.len() as u32;
                vertices.push(v);
                cell_vertex[i + j * n + k * n * n] = Some(idx);
            }
        }
    }

    // ── Step 3: dual quads for sign-changing grid edges ───────────────────────
    //
    // Lookup helper: returns the vertex index for cell (ci, cj, ck), or None
    // if out of range or inactive.  Uses wrapping_sub so boundary indices
    // naturally become usize::MAX which is ≥ n and returns None.
    let cv = |ci: usize, cj: usize, ck: usize| -> Option<u32> {
        if ci >= n || cj >= n || ck >= n { None }
        else { cell_vertex[ci + cj * n + ck * n * n] }
    };

    let mut triangles: Vec<[u32; 3]> = Vec::new();

    // Emit a quad [a, b, c, d] as two triangles; flip reverses the winding.
    macro_rules! emit_quad {
        ($a:expr, $b:expr, $c:expr, $d:expr, $flip:expr) => {
            if let (Some(a), Some(b), Some(c), Some(d)) = ($a, $b, $c, $d) {
                if !$flip {
                    triangles.push([a, b, c]);
                    triangles.push([a, c, d]);
                } else {
                    triangles.push([a, c, b]);
                    triangles.push([a, d, c]);
                }
            }
        };
    }

    // ── X-edges: (i,j,k) → (i+1,j,k) ────────────────────────────────────────
    // Four adjacent cells lie in the YZ plane around the edge.
    // Winding produces outward normal in +X when v0 < 0 (inside at the left
    // vertex) — flip when v0 > 0.
    for k in 0..=n {
        for j in 0..=n {
            for i in 0..n {
                let v0 = values[vi(i,     j, k)];
                let v1 = values[vi(i + 1, j, k)];
                if (v0 < 0.0) == (v1 < 0.0) { continue; }
                emit_quad!(
                    cv(i, j,                 k),
                    cv(i, j,                 k.wrapping_sub(1)),
                    cv(i, j.wrapping_sub(1), k.wrapping_sub(1)),
                    cv(i, j.wrapping_sub(1), k),
                    v0 > 0.0
                );
            }
        }
    }

    // ── Y-edges: (i,j,k) → (i,j+1,k) ────────────────────────────────────────
    for k in 0..=n {
        for j in 0..n {
            for i in 0..=n {
                let v0 = values[vi(i, j,     k)];
                let v1 = values[vi(i, j + 1, k)];
                if (v0 < 0.0) == (v1 < 0.0) { continue; }
                emit_quad!(
                    cv(i,                 j, k),
                    cv(i.wrapping_sub(1), j, k),
                    cv(i.wrapping_sub(1), j, k.wrapping_sub(1)),
                    cv(i,                 j, k.wrapping_sub(1)),
                    v0 < 0.0
                );
            }
        }
    }

    // ── Z-edges: (i,j,k) → (i,j,k+1) ────────────────────────────────────────
    for k in 0..n {
        for j in 0..=n {
            for i in 0..=n {
                let v0 = values[vi(i, j, k)];
                let v1 = values[vi(i, j, k + 1)];
                if (v0 < 0.0) == (v1 < 0.0) { continue; }
                emit_quad!(
                    cv(i,                 j,                 k),
                    cv(i,                 j.wrapping_sub(1), k),
                    cv(i.wrapping_sub(1), j.wrapping_sub(1), k),
                    cv(i.wrapping_sub(1), j,                 k),
                    v0 > 0.0
                );
            }
        }
    }

    Mesh { vertices, triangles }
}
