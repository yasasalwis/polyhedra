//! Regular n-sided prism SDF.
//!
//! A right prism whose cross-section is a regular polygon (n-gon) with a given
//! flat-to-flat distance, extruded to a given height along **Z**.
//!
//! # Axis convention
//! - Axis: Z. Caps at `z = ±height/2`.
//! - The n-gon lies in the XY plane, centred at the origin.
//! - One flat face faces along **+X** (angle 0).
//!
//! # `flat_to_flat`
//! The diameter of the **inscribed** circle (distance between two parallel faces).
//! `apothem = flat_to_flat / 2`.
//!
//! # SDF construction — two steps
//!
//! **Step 1 — 2D n-gon SDF (exact)**
//!
//! Fold the XY point into the canonical sector `[0, π/n]` using the n-fold
//! symmetry, then compute the signed distance to the canonical face segment:
//!
//! ```text
//! sector_angle = |((angle + π/n) mod (2π/n)) − π/n|
//! px = |p_xy| · cos(sector_angle)
//! py = |p_xy| · sin(sector_angle)
//! d_2d = signed dist from (px, py) to segment [(apothem, 0), (apothem, corner)]
//! ```
//!
//! where `corner = apothem · tan(π/n)` is the half-edge width.
//!
//! **Step 2 — Exact extrusion (Quilez)**
//!
//! ```text
//! w   = (d_2d, |p.z| − half_height)
//! SDF = min(max(w.x, w.y), 0) + length(max(w, 0))
//! ```
//!
//! This gives exact Euclidean distance everywhere — inside, on faces/edges/caps,
//! and outside near faces, edges, and corners.

use std::f32::consts::PI;

use glam::{Vec2, Vec3};

use crate::sdf::Sdf;

/// SDF for a regular n-sided prism centred at the origin, axis along Z.
///
/// `ph.Object(ph.PRISM, sides, flat_to_flat, height)` maps to
/// `PrismSdf::new(sides, flat_to_flat, height)`.
pub struct PrismSdf {
    sides: u32,
    apothem: f32, // flat_to_flat / 2  (inradius)
    half_height: f32,
}

impl PrismSdf {
    /// Create from number of sides, flat-to-flat distance, and height.
    ///
    /// # Parameters
    /// - `sides`:        number of sides (3 = triangle, 4 = square, 6 = hexagon, …)
    /// - `flat_to_flat`: distance between two parallel flat faces (diameter of inscribed circle)
    /// - `height`:       total height of the prism along Z
    #[inline]
    pub fn new(sides: u32, flat_to_flat: f32, height: f32) -> Self {
        Self {
            sides,
            apothem: flat_to_flat * 0.5,
            half_height: height * 0.5,
        }
    }

    /// Return `(sides, flat_to_flat, height)`.
    #[inline]
    pub fn dimensions(&self) -> (u32, f32, f32) {
        (self.sides, self.apothem * 2.0, self.half_height * 2.0)
    }

    /// Circumscribed circle radius (centre to vertex).
    ///
    /// `R = apothem / cos(π/n)`
    #[inline]
    pub fn circumradius(&self) -> f32 {
        let han = PI / self.sides as f32;
        self.apothem / han.cos()
    }
}

impl Sdf for PrismSdf {
    /// Exact signed distance to the prism surface.
    fn distance(&self, p: Vec3) -> f32 {
        let n = self.sides as f32;
        let a = self.apothem;
        let hh = self.half_height;
        let han = PI / n; // π/n  (half sector angle)
        let an = 2.0 * han; // 2π/n (full sector angle)
        let corner = a * han.tan(); // half-edge width: apothem · tan(π/n)

        // ── Step 1: fold into canonical sector [0, π/n] ──────────────────
        let r_xy = Vec2::new(p.x, p.y).length();
        let angle = p.y.atan2(p.x); // in (−π, π]
        // Rotate to nearest face, fold to [0, π/n] by symmetry.
        let sector_angle = ((angle + han).rem_euclid(an) - han).abs();
        let px = r_xy * sector_angle.cos();
        let py = r_xy * sector_angle.sin();

        // ── Step 2: 2D n-gon SDF ─────────────────────────────────────────
        // Distance from (px, py) to the segment [(a, 0), (a, corner)].
        // The segment lies along the canonical face of the n-gon.
        let ny = py.clamp(0.0, corner); // nearest y on the segment
        let dx = px - a;
        let dy = py - ny;
        let raw = dx.hypot(dy);
        // Interior: px ≤ apothem AND py ≤ corner (by construction of the fold).
        let d_2d = if px <= a && py <= corner { -raw } else { raw };

        // ── Step 3: exact extrusion (Quilez) ─────────────────────────────
        let d_z = p.z.abs() - hh;
        let w = Vec2::new(d_2d, d_z);
        w.x.max(w.y).min(0.0) + w.max(Vec2::ZERO).length()
    }
    // Normal: use default finite-difference implementation from Sdf trait.
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    // Hexagonal prism: n=6, flat_to_flat=10, height=20.
    // apothem=5, corner=5/√3≈2.887, R=10/√3≈5.774, half_height=10.
    fn hex() -> PrismSdf {
        PrismSdf::new(6, 10.0, 20.0)
    }

    // Triangular prism: n=3, flat_to_flat=10, height=20.
    // apothem=5, corner=5·√3≈8.66, R=10, half_height=10.
    fn tri() -> PrismSdf {
        PrismSdf::new(3, 10.0, 20.0)
    }

    // ── Accessors ─────────────────────────────────────────────────────────

    #[test]
    fn dimensions_round_trip() {
        let (s, f, h) = hex().dimensions();
        assert_eq!(s, 6);
        assert_abs_diff_eq!(f, 10.0, epsilon = 1e-5);
        assert_abs_diff_eq!(h, 20.0, epsilon = 1e-5);
    }

    #[test]
    fn circumradius_hex() {
        // R = 5 / cos(π/6) = 5 / (√3/2) = 10/√3 ≈ 5.7735.
        let expected = 10.0_f32 / 3.0_f32.sqrt();
        assert_abs_diff_eq!(hex().circumradius(), expected, epsilon = 1e-4);
    }

    // ── Interior ──────────────────────────────────────────────────────────

    #[test]
    fn hex_center_inside() {
        assert!(hex().distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn hex_center_distance_equals_negative_apothem() {
        // Nearest face is 5 mm away → d = -5.
        assert_abs_diff_eq!(hex().distance(Vec3::ZERO), -5.0, epsilon = 1e-4);
    }

    #[test]
    fn tri_center_inside() {
        assert!(tri().distance(Vec3::ZERO) < 0.0);
    }

    // ── Surface — flat face centres ────────────────────────────────────────

    #[test]
    fn hex_face_center_on_surface() {
        // The face 0 centre is at (apothem, 0, 0) = (5, 0, 0).
        let d = hex().distance(Vec3::new(5.0, 0.0, 0.0));
        assert!(d.abs() < 1e-4, "hex face centre: {d}");
    }

    #[test]
    fn hex_all_face_centres_on_surface() {
        let h = hex();
        for k in 0..6u32 {
            let angle = k as f32 * std::f32::consts::TAU / 6.0;
            let p = Vec3::new(5.0 * angle.cos(), 5.0 * angle.sin(), 0.0);
            let d = h.distance(p);
            assert!(d.abs() < 1e-4, "face {k} centre ({p:?}): {d}");
        }
    }

    // ── Surface — corner vertices ──────────────────────────────────────────

    #[test]
    fn hex_corner_vertices_on_surface() {
        let h = hex();
        let r = h.circumradius(); // ≈ 5.7735
        // Vertices are at angles 30°, 90°, 150°, 210°, 270°, 330°.
        for k in 0..6u32 {
            let angle = (k as f32 + 0.5) * std::f32::consts::TAU / 6.0;
            let p = Vec3::new(r * angle.cos(), r * angle.sin(), 0.0);
            let d = h.distance(p);
            assert!(d.abs() < 1e-4, "vertex {k} ({p:?}): expected ~0, got {d}");
        }
    }

    #[test]
    fn tri_corner_vertex_on_surface() {
        let t = tri();
        let r = t.circumradius(); // = 10.0
        // One vertex directly above in +X direction for face at angle 0.
        // For n=3, vertices are at 60°, 180°, 300°.
        let angle = std::f32::consts::PI / 3.0; // 60°
        let p = Vec3::new(r * angle.cos(), r * angle.sin(), 0.0);
        let d = t.distance(p);
        assert!(d.abs() < 1e-4, "tri vertex ({p:?}): {d}");
    }

    // ── Surface — caps ────────────────────────────────────────────────────

    #[test]
    fn hex_top_cap_on_surface() {
        let d = hex().distance(Vec3::new(0.0, 0.0, 10.0));
        assert!(d.abs() < 1e-4, "top cap: {d}");
    }

    #[test]
    fn hex_bottom_cap_on_surface() {
        let d = hex().distance(Vec3::new(0.0, 0.0, -10.0));
        assert!(d.abs() < 1e-4, "bottom cap: {d}");
    }

    // ── Exterior ──────────────────────────────────────────────────────────

    #[test]
    fn hex_outside_radially_from_face() {
        // 3 mm past the +X face: p = (8, 0, 0).
        assert_abs_diff_eq!(
            hex().distance(Vec3::new(8.0, 0.0, 0.0)),
            3.0,
            epsilon = 1e-4
        );
    }

    #[test]
    fn hex_outside_axially_above() {
        // 4 mm above the top cap: p = (0, 0, 14).
        assert_abs_diff_eq!(
            hex().distance(Vec3::new(0.0, 0.0, 14.0)),
            4.0,
            epsilon = 1e-4
        );
    }

    #[test]
    fn hex_outside_both_corner() {
        // Outside both radially (d=3) and axially (d=4) → d = sqrt(9+16) = 5.
        assert_abs_diff_eq!(
            hex().distance(Vec3::new(8.0, 0.0, 14.0)),
            5.0,
            epsilon = 1e-3,
        );
    }

    // ── n=4 square prism ──────────────────────────────────────────────────

    #[test]
    fn square_prism_face_on_surface() {
        // n=4, flat_to_flat=10, height=10. Face at (5, 0, 0).
        let sq = PrismSdf::new(4, 10.0, 10.0);
        let d = sq.distance(Vec3::new(5.0, 0.0, 0.0));
        assert!(d.abs() < 1e-4, "square face: {d}");
    }

    #[test]
    fn square_prism_center_distance() {
        // apothem=5, nearest face at 5 mm.
        let sq = PrismSdf::new(4, 10.0, 10.0);
        assert_abs_diff_eq!(sq.distance(Vec3::ZERO), -5.0, epsilon = 1e-4);
    }

    // ── Trait object ──────────────────────────────────────────────────────

    #[test]
    fn prism_as_sdf_node() {
        use crate::sdf::SdfNode;
        let node: SdfNode = Box::new(PrismSdf::new(6, 10.0, 20.0));
        assert!(node.distance(Vec3::ZERO) < 0.0);
        assert!(node.distance(Vec3::new(30.0, 0.0, 0.0)) > 0.0);
    }
}
