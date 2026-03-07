//! Capped cylinder SDF.
//!
//! Uses the Inigo Quilez exact capped-cylinder formula.
//! The cylinder is centred at the origin with its axis along **Z**.
//!
//! Distance formula:
//!
//!   lateral = length(p.xy) - radius
//!   axial   = |p.z| - height/2
//!   d       = (lateral, axial)
//!   sdf     = min(max(lateral, axial), 0) + length(max(d, 0))
//!
//! This yields the exact Euclidean distance everywhere — inside the solid
//! (negative), on the surface (zero), and outside (positive).

use glam::Vec2;
use glam::Vec3;

use crate::sdf::Sdf;

/// SDF for a capped cylinder centred at the origin, axis along Z.
///
/// `ph.Object(ph.CYLINDER, radius, height)` maps to `CylinderSdf::new(radius, height)`.
///
/// # Coordinate convention
///
/// - Axis: Z (the cylinder stands "upright").
/// - Spans `[-height/2, +height/2]` along Z.
/// - Spans a circle of `radius` in the XY plane.
///
/// To orient or position it differently, wrap in a
/// [`crate::sdf::TransformedSdf`] or [`crate::sdf::transform::Rotate`].
pub struct CylinderSdf {
    radius:       f32,
    half_height:  f32,
}

impl CylinderSdf {
    /// Create from radius and total height.
    ///
    /// # Panics
    /// Does not panic; zero or negative values produce degenerate (but valid)
    /// SDFs — the geometry kernel will catch invalid dimensions at a higher level.
    #[inline]
    pub fn new(radius: f32, height: f32) -> Self {
        Self {
            radius,
            half_height: height * 0.5,
        }
    }

    /// Return `(radius, height)`.
    #[inline]
    pub fn dimensions(&self) -> (f32, f32) {
        (self.radius, self.half_height * 2.0)
    }
}

impl Sdf for CylinderSdf {
    /// Exact signed distance from `p` to the capped cylinder surface.
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        // Radial distance from the Z axis, minus radius.
        let lateral = Vec2::new(p.x, p.y).length() - self.radius;
        // Axial distance from the half-height planes.
        let axial = p.z.abs() - self.half_height;

        // Quilez 2D box distance in (lateral, axial) space:
        //   - Inside both slabs → both components negative → max is negative → interior.
        //   - Outside one slab  → distance to that face.
        //   - Outside both      → distance to the rim edge.
        let d = Vec2::new(lateral, axial);
        d.x.max(d.y).min(0.0) + d.max(Vec2::ZERO).length()
    }

    /// Analytical normal — avoids finite-difference error on flat caps.
    fn normal(&self, p: Vec3) -> Vec3 {
        let lateral = Vec2::new(p.x, p.y).length() - self.radius;
        let axial   = p.z.abs() - self.half_height;

        if axial > lateral {
            // Closest feature is a flat cap.
            Vec3::new(0.0, 0.0, p.z.signum())
        } else {
            // Closest feature is the curved side.
            let r = Vec2::new(p.x, p.y).length().max(f32::EPSILON);
            Vec3::new(p.x / r, p.y / r, 0.0)
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    /// r=5, h=20 → half_height=10.
    fn cyl() -> CylinderSdf {
        CylinderSdf::new(5.0, 20.0)
    }

    // ── Interior ──────────────────────────────────────────────────────────

    #[test]
    fn center_is_inside() {
        assert!(cyl().distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn center_distance_is_negative_min_of_radius_and_half_height() {
        // min(5, 10) = 5 → distance at centre = -5.
        assert_abs_diff_eq!(cyl().distance(Vec3::ZERO), -5.0, epsilon = 1e-5);
    }

    // ── Surface ───────────────────────────────────────────────────────────

    #[test]
    fn curved_surface_is_on_surface() {
        // Any point at radius 5, |z| < 10.
        let c = cyl();
        for z in [-9.0_f32, 0.0, 9.0] {
            let d = c.distance(Vec3::new(5.0, 0.0, z));
            assert!(d.abs() < 1e-5, "z={z}: expected ~0, got {d}");
        }
    }

    #[test]
    fn top_cap_center_is_on_surface() {
        assert_abs_diff_eq!(cyl().distance(Vec3::new(0.0, 0.0, 10.0)), 0.0, epsilon = 1e-5);
    }

    #[test]
    fn bottom_cap_center_is_on_surface() {
        assert_abs_diff_eq!(cyl().distance(Vec3::new(0.0, 0.0, -10.0)), 0.0, epsilon = 1e-5);
    }

    #[test]
    fn rim_edge_is_on_surface() {
        // The rim is where the curved surface meets a cap: (5, 0, ±10).
        let c = cyl();
        assert_abs_diff_eq!(c.distance(Vec3::new(5.0, 0.0,  10.0)), 0.0, epsilon = 1e-5);
        assert_abs_diff_eq!(c.distance(Vec3::new(5.0, 0.0, -10.0)), 0.0, epsilon = 1e-5);
    }

    // ── Exterior ──────────────────────────────────────────────────────────

    #[test]
    fn outside_radially_is_positive() {
        // 3 mm past the curved surface, mid-height.
        assert_abs_diff_eq!(cyl().distance(Vec3::new(8.0, 0.0, 0.0)), 3.0, epsilon = 1e-5);
    }

    #[test]
    fn outside_axially_is_positive() {
        // 4 mm above the top cap, on-axis.
        assert_abs_diff_eq!(cyl().distance(Vec3::new(0.0, 0.0, 14.0)), 4.0, epsilon = 1e-5);
    }

    #[test]
    fn outside_corner_is_correct() {
        // 3 mm past radius AND 4 mm past cap → distance to the rim edge = sqrt(9+16)=5.
        assert_abs_diff_eq!(
            cyl().distance(Vec3::new(8.0, 0.0, 14.0)),
            5.0,
            epsilon = 1e-4,
        );
    }

    // ── Non-uniform ───────────────────────────────────────────────────────

    #[test]
    fn narrow_tall_cylinder() {
        // r=2, h=100 → at centre, nearest face is the curved side: d = -2.
        let c = CylinderSdf::new(2.0, 100.0);
        assert_abs_diff_eq!(c.distance(Vec3::ZERO), -2.0, epsilon = 1e-5);
    }

    #[test]
    fn wide_flat_disk() {
        // r=50, h=2 → at centre, nearest face is the cap: d = -1.
        let c = CylinderSdf::new(50.0, 2.0);
        assert_abs_diff_eq!(c.distance(Vec3::ZERO), -1.0, epsilon = 1e-5);
    }

    #[test]
    fn dimensions_round_trip() {
        let c = CylinderSdf::new(7.5, 30.0);
        let (r, h) = c.dimensions();
        assert_abs_diff_eq!(r, 7.5,  epsilon = 1e-5);
        assert_abs_diff_eq!(h, 30.0, epsilon = 1e-5);
    }

    // ── Normal ────────────────────────────────────────────────────────────

    #[test]
    fn normal_on_curved_surface_points_radially() {
        let n = cyl().normal(Vec3::new(5.0, 0.0, 0.0));
        assert_abs_diff_eq!(n.x, 1.0, epsilon = 1e-4);
        assert_abs_diff_eq!(n.y, 0.0, epsilon = 1e-4);
        assert_abs_diff_eq!(n.z, 0.0, epsilon = 1e-4);
    }

    #[test]
    fn normal_on_top_cap_points_up() {
        let n = cyl().normal(Vec3::new(0.0, 0.0, 10.0));
        assert_abs_diff_eq!(n.z, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn normal_on_bottom_cap_points_down() {
        let n = cyl().normal(Vec3::new(0.0, 0.0, -10.0));
        assert_abs_diff_eq!(n.z, -1.0, epsilon = 1e-4);
    }

    // ── Trait object ──────────────────────────────────────────────────────

    #[test]
    fn cylinder_as_sdf_node() {
        use crate::sdf::SdfNode;
        let node: SdfNode = Box::new(CylinderSdf::new(5.0, 20.0));
        assert!(node.distance(Vec3::ZERO) < 0.0);
        assert!(node.distance(Vec3::new(20.0, 0.0, 0.0)) > 0.0);
    }
}
