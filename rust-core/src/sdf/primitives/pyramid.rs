//! Rectangular pyramid SDF.
//!
//! A four-sided pyramid with a rectangular base, centred at the origin.
//! The apex points along **+Z** and the base lies in the Z = -height/2 plane.
//!
//! # SDF construction
//!
//! The pyramid is a convex polytope bounded by five half-spaces:
//!
//! 1. **Bottom cap** — plane z = -half_height, outward normal (0, 0, -1).
//! 2. **±X slant faces** — two symmetric faces meeting the apex, outward normal
//!    direction `(2·hh, 0, hx)` (unnormalised, where hh = half_height, hx = half_width).
//! 3. **±Y slant faces** — two symmetric faces, outward normal direction `(0, 2·hh, hy)`.
//!
//! The SDF is `max(d_base, d_x, d_y)` — exact Euclidean distance for points whose
//! nearest surface feature is a face interior.  Near edges and the apex the magnitude
//! is a conservative bound (correct sign, slightly underestimated distance).
//! This is the standard approach used in SDF-based geometry kernels and is
//! perfectly adequate for Dual Contouring meshing.

use glam::Vec3;

use crate::sdf::Sdf;

/// SDF for a rectangular pyramid centred at the origin, apex along +Z.
///
/// `ph.Object(ph.PYRAMID, base_width, base_depth, height)` maps to
/// `PyramidSdf::new(base_width, base_depth, height)`.
///
/// # Coordinate convention
/// - Apex at `(0, 0,  height/2)`.
/// - Base centre at `(0, 0, -height/2)`.
/// - Base corners at `(±width/2, ±depth/2, -height/2)`.
pub struct PyramidSdf {
    half_width:  f32,   // hx
    half_depth:  f32,   // hy
    half_height: f32,   // hh
}

impl PyramidSdf {
    /// Create from full base width, base depth, and height.
    #[inline]
    pub fn new(base_width: f32, base_depth: f32, height: f32) -> Self {
        Self {
            half_width:  base_width  * 0.5,
            half_depth:  base_depth  * 0.5,
            half_height: height      * 0.5,
        }
    }

    /// Return `(base_width, base_depth, height)`.
    #[inline]
    pub fn dimensions(&self) -> (f32, f32, f32) {
        (self.half_width * 2.0, self.half_depth * 2.0, self.half_height * 2.0)
    }
}

impl Sdf for PyramidSdf {
    fn distance(&self, p: Vec3) -> f32 {
        let hx = self.half_width;
        let hy = self.half_depth;
        let hh = self.half_height;

        // Exploit XY symmetry — fold into the first quadrant.
        let px = p.x.abs();
        let py = p.y.abs();

        // ── Bottom cap ────────────────────────────────────────────────────
        // Outward normal (0, 0, -1), plane at z = -hh.
        let d_base = -p.z - hh;

        // ── ±X slant faces ────────────────────────────────────────────────
        // Each face passes through the apex (0, 0, hh) and the base edge at x = hx.
        // Unnormalised outward normal: (2·hh, 0, hx).
        let nx_len = (4.0 * hh * hh + hx * hx).sqrt();
        let d_x = (2.0 * hh * px + hx * p.z - hh * hx) / nx_len;

        // ── ±Y slant faces ────────────────────────────────────────────────
        // Unnormalised outward normal: (0, 2·hh, hy).
        let ny_len = (4.0 * hh * hh + hy * hy).sqrt();
        let d_y = (2.0 * hh * py + hy * p.z - hh * hy) / ny_len;

        // ── Convex intersection ────────────────────────────────────────────
        // Inside all half-spaces → negative (interior).
        // Outside at least one → positive (exterior).
        d_base.max(d_x).max(d_y)
    }
    // Normal: use default finite-difference implementation.
    // The analytical gradient for this SDF is piecewise constant per face and
    // discontinuous at edges; finite differences give a smooth approximation
    // suitable for normal estimation in DC meshing.
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    /// 10 × 10 base, 10 tall.  hx=hy=5, hh=5.  Apex at (0,0,5).
    fn square_pyr() -> PyramidSdf { PyramidSdf::new(10.0, 10.0, 10.0) }

    /// 20 × 10 base, 10 tall.  hx=10, hy=5, hh=5.
    fn rect_pyr() -> PyramidSdf { PyramidSdf::new(20.0, 10.0, 10.0) }

    // ── Key surface points ─────────────────────────────────────────────────

    #[test]
    fn apex_is_on_surface() {
        let d = square_pyr().distance(Vec3::new(0.0, 0.0, 5.0));
        assert!(d.abs() < 1e-5, "apex: expected ~0, got {d}");
    }

    #[test]
    fn base_center_is_on_surface() {
        let d = square_pyr().distance(Vec3::new(0.0, 0.0, -5.0));
        assert!(d.abs() < 1e-5, "base centre: expected ~0, got {d}");
    }

    #[test]
    fn base_corner_is_on_surface() {
        let d = square_pyr().distance(Vec3::new(5.0, 5.0, -5.0));
        assert!(d.abs() < 1e-5, "base corner: expected ~0, got {d}");
    }

    #[test]
    fn base_edge_midpoint_is_on_surface() {
        // Mid of the +X base edge: (5, 0, -5).
        let d = square_pyr().distance(Vec3::new(5.0, 0.0, -5.0));
        assert!(d.abs() < 1e-5, "base edge mid: expected ~0, got {d}");
    }

    // ── Interior ──────────────────────────────────────────────────────────

    #[test]
    fn center_is_inside() {
        assert!(square_pyr().distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn near_base_is_inside() {
        assert!(square_pyr().distance(Vec3::new(0.0, 0.0, -4.9)) < 0.0);
    }

    // ── Exterior — face regions (formula is exact here) ───────────────────

    #[test]
    fn below_base_on_axis() {
        // On axis, directly below base: nearest feature is the base plane.
        assert_abs_diff_eq!(
            square_pyr().distance(Vec3::new(0.0, 0.0, -9.0)),
            4.0,
            epsilon = 1e-4,
        );
    }

    #[test]
    fn outside_x_slant_face_region() {
        // A point outside the +X slant face, away from edges.
        // At z=0, the slant face has radius 2.5 (half of hx at mid-height).
        // Point (6, 0, 0) is outside the face.
        let d = square_pyr().distance(Vec3::new(6.0, 0.0, 0.0));
        assert!(d > 0.0, "should be outside: {d}");
    }

    #[test]
    fn outside_y_slant_face_region() {
        let d = square_pyr().distance(Vec3::new(0.0, 6.0, 0.0));
        assert!(d > 0.0, "should be outside: {d}");
    }

    // ── Rectangular pyramid ────────────────────────────────────────────────

    #[test]
    fn rect_apex_is_on_surface() {
        let d = rect_pyr().distance(Vec3::new(0.0, 0.0, 5.0));
        assert!(d.abs() < 1e-5, "rect apex: {d}");
    }

    #[test]
    fn rect_base_corner_is_on_surface() {
        let d = rect_pyr().distance(Vec3::new(10.0, 5.0, -5.0));
        assert!(d.abs() < 1e-5, "rect base corner: {d}");
    }

    #[test]
    fn rect_center_is_inside() {
        assert!(rect_pyr().distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn rect_below_base_distance() {
        assert_abs_diff_eq!(
            rect_pyr().distance(Vec3::new(0.0, 0.0, -8.0)),
            3.0,
            epsilon = 1e-4,
        );
    }

    // ── Dimensions ────────────────────────────────────────────────────────

    #[test]
    fn dimensions_round_trip() {
        let p = PyramidSdf::new(8.0, 6.0, 12.0);
        let (w, d, h) = p.dimensions();
        assert_abs_diff_eq!(w, 8.0,  epsilon = 1e-5);
        assert_abs_diff_eq!(d, 6.0,  epsilon = 1e-5);
        assert_abs_diff_eq!(h, 12.0, epsilon = 1e-5);
    }

    // ── Trait object ──────────────────────────────────────────────────────

    #[test]
    fn pyramid_as_sdf_node() {
        use crate::sdf::SdfNode;
        let node: SdfNode = Box::new(PyramidSdf::new(10.0, 10.0, 10.0));
        assert!(node.distance(Vec3::ZERO) < 0.0);
        assert!(node.distance(Vec3::new(0.0, 0.0, 50.0)) > 0.0);
    }
}
