//! Truncated cone (frustum) SDF.
//!
//! Handles both the standard pointed cone (`top_radius = 0`) and the general
//! truncated cone (frustum) with distinct base and top radii.
//!
//! Uses the Inigo Quilez exact capped-cone formula, adapted for the Z axis.
//!
//! Axis convention: Z. Bottom face at z = -height/2 with `base_radius`;
//! top face at z = +height/2 with `top_radius`.

use glam::Vec2;
use glam::Vec3;

use crate::sdf::Sdf;

/// SDF for a (truncated) cone centred at the origin, axis along Z.
///
/// `ph.Object(ph.CONE, base_radius, top_radius, height)` maps to
/// `ConeSdf::new(base_radius, top_radius, height)`.
///
/// Setting `top_radius = 0.0` gives a standard pointed cone.
/// Setting `base_radius == top_radius` gives a cylinder (use `CylinderSdf` for that).
pub struct ConeSdf {
    base_radius: f32,   // radius at z = -half_height
    top_radius:  f32,   // radius at z = +half_height
    half_height: f32,
}

impl ConeSdf {
    /// Create from base radius, top radius, and total height.
    #[inline]
    pub fn new(base_radius: f32, top_radius: f32, height: f32) -> Self {
        Self {
            base_radius,
            top_radius,
            half_height: height * 0.5,
        }
    }

    /// Return `(base_radius, top_radius, height)`.
    #[inline]
    pub fn dimensions(&self) -> (f32, f32, f32) {
        (self.base_radius, self.top_radius, self.half_height * 2.0)
    }
}

impl Sdf for ConeSdf {
    /// Exact signed distance using the Quilez capped-cone formula.
    ///
    /// Works for:
    /// - Pointed cones  (`top_radius = 0`)
    /// - Frustums       (`top_radius > 0`)
    /// - Inverted cones (`top_radius > base_radius`)
    fn distance(&self, p: Vec3) -> f32 {
        let h  = self.half_height;
        let r1 = self.base_radius;   // at z = -h
        let r2 = self.top_radius;    // at z = +h

        // Collapse to 2-D: (radial distance from Z axis, z coordinate).
        let q = Vec2::new(Vec2::new(p.x, p.y).length(), p.z);

        // Key vectors for the slant edge in 2-D.
        let k1 = Vec2::new(r2, h);
        let k2 = Vec2::new(r2 - r1, 2.0 * h);

        // Distance to the two flat caps (via a 2-D box test).
        let ca = Vec2::new(
            q.x - q.x.min(if q.y < 0.0 { r1 } else { r2 }),
            q.y.abs() - h,
        );

        // Distance to the slant edge (clamped projection).
        let t  = ((k1 - q).dot(k2) / k2.dot(k2)).clamp(0.0, 1.0);
        let cb = q - k1 + k2 * t;

        // Sign: negative (inside) when both ca and cb say "inside".
        let s = if cb.x < 0.0 && ca.y < 0.0 { -1.0_f32 } else { 1.0_f32 };

        s * ca.dot(ca).min(cb.dot(cb)).sqrt()
    }
    // Normal: use default finite-difference implementation from Sdf trait.
    // The slant-surface analytical gradient is non-trivial; finite differences
    // are accurate to ~1e-4 which is sufficient for mesh generation.
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    // ── Pointed cone: base_radius=5, top_radius=0, height=10 ──────────────
    fn pointed() -> ConeSdf { ConeSdf::new(5.0, 0.0, 10.0) }

    // ── Frustum: base_radius=6, top_radius=3, height=10 ───────────────────
    fn frustum() -> ConeSdf { ConeSdf::new(6.0, 3.0, 10.0) }

    // ── Interior ──────────────────────────────────────────────────────────

    #[test]
    fn pointed_center_is_inside() {
        // z=0 midpoint: radius at mid = 2.5 (linear interp), point at r=0 → inside.
        assert!(pointed().distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn frustum_center_is_inside() {
        // z=0: radius = (6+3)/2 = 4.5; point at r=0 → inside.
        assert!(frustum().distance(Vec3::ZERO) < 0.0);
    }

    // ── Pointed cone surface ───────────────────────────────────────────────

    #[test]
    fn pointed_tip_is_on_surface() {
        // Tip at z = +5 (= +half_height), top_radius = 0.
        let d = pointed().distance(Vec3::new(0.0, 0.0, 5.0));
        assert!(d.abs() < 1e-4, "tip: expected ~0, got {d}");
    }

    #[test]
    fn pointed_base_center_is_on_surface() {
        // Base cap centre at z = -5.
        let d = pointed().distance(Vec3::new(0.0, 0.0, -5.0));
        assert!(d.abs() < 1e-4, "base centre: expected ~0, got {d}");
    }

    #[test]
    fn pointed_base_rim_is_on_surface() {
        // Base rim: (r=5, z=-5).
        let d = pointed().distance(Vec3::new(5.0, 0.0, -5.0));
        assert!(d.abs() < 1e-4, "base rim: expected ~0, got {d}");
    }

    // ── Frustum surface ────────────────────────────────────────────────────

    #[test]
    fn frustum_base_center_is_on_surface() {
        let d = frustum().distance(Vec3::new(0.0, 0.0, -5.0));
        assert!(d.abs() < 1e-4, "frustum base: expected ~0, got {d}");
    }

    #[test]
    fn frustum_top_center_is_on_surface() {
        let d = frustum().distance(Vec3::new(0.0, 0.0, 5.0));
        assert!(d.abs() < 1e-4, "frustum top: expected ~0, got {d}");
    }

    #[test]
    fn frustum_base_rim_is_on_surface() {
        let d = frustum().distance(Vec3::new(6.0, 0.0, -5.0));
        assert!(d.abs() < 1e-4, "frustum base rim: expected ~0, got {d}");
    }

    #[test]
    fn frustum_top_rim_is_on_surface() {
        let d = frustum().distance(Vec3::new(3.0, 0.0, 5.0));
        assert!(d.abs() < 1e-4, "frustum top rim: expected ~0, got {d}");
    }

    // ── Exterior ──────────────────────────────────────────────────────────

    #[test]
    fn pointed_outside_axially_above_tip() {
        // 3 mm above the tip (z = 5 + 3 = 8), on axis.
        assert_abs_diff_eq!(pointed().distance(Vec3::new(0.0, 0.0, 8.0)), 3.0, epsilon = 1e-4);
    }

    #[test]
    fn pointed_outside_below_base() {
        // 4 mm below the base (z = -5 - 4 = -9), on axis.
        assert_abs_diff_eq!(pointed().distance(Vec3::new(0.0, 0.0, -9.0)), 4.0, epsilon = 1e-4);
    }

    #[test]
    fn frustum_outside_above_top() {
        assert_abs_diff_eq!(frustum().distance(Vec3::new(0.0, 0.0, 8.0)), 3.0, epsilon = 1e-4);
    }

    #[test]
    fn frustum_outside_below_base() {
        assert_abs_diff_eq!(frustum().distance(Vec3::new(0.0, 0.0, -8.0)), 3.0, epsilon = 1e-4);
    }

    // ── Dimensions ────────────────────────────────────────────────────────

    #[test]
    fn dimensions_round_trip() {
        let c = ConeSdf::new(8.0, 3.0, 20.0);
        let (b, t, h) = c.dimensions();
        assert_abs_diff_eq!(b, 8.0,  epsilon = 1e-5);
        assert_abs_diff_eq!(t, 3.0,  epsilon = 1e-5);
        assert_abs_diff_eq!(h, 20.0, epsilon = 1e-5);
    }

    // ── Trait object ──────────────────────────────────────────────────────

    #[test]
    fn cone_as_sdf_node() {
        use crate::sdf::SdfNode;
        let node: SdfNode = Box::new(ConeSdf::new(5.0, 0.0, 10.0));
        assert!(node.distance(Vec3::ZERO) < 0.0);
        assert!(node.distance(Vec3::new(20.0, 0.0, 0.0)) > 0.0);
    }
}
