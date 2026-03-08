//! Quadratic Error Function (QEF) solver for Dual Contouring.
//!
//! Minimises:
//!
//! ```text
//! E(x) = Σ (nᵢ · (x − pᵢ))²  +  λ · |x − c|²
//! ```
//!
//! where `(pᵢ, nᵢ)` are edge-crossing points / normals, `c` is the cell
//! centre, and `λ` is a regularisation weight that prevents degenerate solves
//! in flat or planar regions.
//!
//! The problem reduces to the 3 × 3 symmetric linear system
//!
//! ```text
//! (AᵀA + λI) x = Aᵀb + λc
//! ```
//!
//! solved here by Gaussian elimination with partial pivoting.

use glam::Vec3;

// ── QEF accumulator ───────────────────────────────────────────────────────────

/// Accumulates the QEF normal-equation terms for one active DC cell.
pub struct Qef {
    ata: [[f64; 3]; 3], // AᵀA  (symmetric 3 × 3)
    atb: [f64; 3],      // Aᵀb
}

impl Qef {
    pub fn new() -> Self {
        Self { ata: [[0.0; 3]; 3], atb: [0.0; 3] }
    }

    /// Add one edge-crossing constraint: surface point `p` with outward normal `n`.
    pub fn add(&mut self, p: Vec3, n: Vec3) {
        let p = [p.x as f64, p.y as f64, p.z as f64];
        let n = [n.x as f64, n.y as f64, n.z as f64];
        let b = n[0] * p[0] + n[1] * p[1] + n[2] * p[2];
        for i in 0..3 {
            for j in 0..3 {
                self.ata[i][j] += n[i] * n[j];
            }
            self.atb[i] += n[i] * b;
        }
    }

    /// Solve for the vertex position, regularised toward `cell_center`.
    ///
    /// `lambda` ≈ 1e-3.  Larger → pulls toward the cell centre (safer but
    /// blunter); smaller → sharper features but less stable near flat regions.
    ///
    /// Falls back to `cell_center` if the system is (near-)singular.
    pub fn solve(&self, cell_center: Vec3, lambda: f64) -> Vec3 {
        let c = [cell_center.x as f64, cell_center.y as f64, cell_center.z as f64];
        let mut m = self.ata;
        let mut rhs = self.atb;
        for i in 0..3 {
            m[i][i] += lambda;
            rhs[i] += lambda * c[i];
        }
        solve3(m, rhs)
            .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
            .unwrap_or(cell_center)
    }
}

// ── 3 × 3 Gaussian elimination ────────────────────────────────────────────────

/// Solve `m · x = b` via Gaussian elimination with partial pivoting.
/// Returns `None` if the matrix is (near-)singular.
fn solve3(m: [[f64; 3]; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let mut a = [
        [m[0][0], m[0][1], m[0][2], b[0]],
        [m[1][0], m[1][1], m[1][2], b[1]],
        [m[2][0], m[2][1], m[2][2], b[2]],
    ];

    for col in 0..3 {
        // Partial pivot: find row with largest absolute value in this column.
        let pivot_row = (col..3)
            .max_by(|&r1, &r2| {
                a[r1][col].abs().partial_cmp(&a[r2][col].abs()).unwrap()
            })
            .unwrap();
        a.swap(col, pivot_row);

        let pivot = a[col][col];
        if pivot.abs() < 1e-10 {
            return None;
        }
        for j in col..4 {
            a[col][j] /= pivot;
        }
        for row in 0..3 {
            if row != col {
                let factor = a[row][col];
                for j in col..4 {
                    a[row][j] -= factor * a[col][j];
                }
            }
        }
    }

    Some([a[0][3], a[1][3], a[2][3]])
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn solve3_identity() {
        let m = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let b = [3.0, 5.0, 7.0];
        let x = solve3(m, b).unwrap();
        assert_abs_diff_eq!(x[0], 3.0, epsilon = 1e-9);
        assert_abs_diff_eq!(x[1], 5.0, epsilon = 1e-9);
        assert_abs_diff_eq!(x[2], 7.0, epsilon = 1e-9);
    }

    #[test]
    fn solve3_singular_returns_none() {
        let m = [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 0.0, 1.0]];
        let b = [1.0, 2.0, 1.0];
        assert!(solve3(m, b).is_none());
    }

    /// A 3-plane QEF with normals along each axis should place the vertex at
    /// the intersection point regardless of the regularisation weight (for
    /// a well-conditioned system).
    #[test]
    fn qef_three_axis_planes() {
        // Three planes: x=1, y=2, z=3.
        let mut qef = Qef::new();
        qef.add(Vec3::new(1.0, 0.0, 0.0), Vec3::X);
        qef.add(Vec3::new(0.0, 2.0, 0.0), Vec3::Y);
        qef.add(Vec3::new(0.0, 0.0, 3.0), Vec3::Z);
        let v = qef.solve(Vec3::ZERO, 1e-6);
        assert_abs_diff_eq!(v.x, 1.0, epsilon = 1e-3);
        assert_abs_diff_eq!(v.y, 2.0, epsilon = 1e-3);
        assert_abs_diff_eq!(v.z, 3.0, epsilon = 1e-3);
    }

    /// With heavy regularisation the result should be pulled toward the cell
    /// centre.
    #[test]
    fn qef_regularisation_pulls_to_center() {
        let mut qef = Qef::new();
        // Single plane at x = 10 — under-constrained without regularisation.
        qef.add(Vec3::new(10.0, 0.0, 0.0), Vec3::X);
        let center = Vec3::new(0.5, 0.5, 0.5);
        // With strong regularisation the result should be near the center.
        let v = qef.solve(center, 1e3);
        assert!(v.distance(center) < 1.0, "expected near center, got {v:?}");
    }
}
