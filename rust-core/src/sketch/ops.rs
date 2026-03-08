//! 3D operations on 2D sketch profiles.
//!
//! Both operations are exact Euclidean SDFs and agree identically with the
//! corresponding built-in primitives:
//!
//! | Sketch expression                          | Equivalent primitive              |
//! |--------------------------------------------|-----------------------------------|
//! | `Extrude { Circle(r),     height: h }`     | `CylinderSdf::new(r, h)`          |
//! | `Extrude { Rect(w,d),     height: h }`     | `CubeSdf::new(w, d, h)`           |
//! | `Extrude { RegPoly(n,a),  height: h }`     | `PrismSdf::new(n, 2a, h)`         |
//! | `Revolve { Circle(minor), Z, major }`      | `TorusSdf::new(major, minor)`     |

use glam::{Vec2, Vec3};

use crate::sdf::Sdf;

use super::Sdf2dNode;

// ── ExtrudeNode ───────────────────────────────────────────────────────────────

/// Extrude a 2D profile along the **Z axis** using the Quilez exact extrusion
/// formula.
///
/// The profile lives in the XY plane.  The resulting solid spans
/// `z ∈ [−height/2, +height/2]`.
///
/// ```text
/// w   = (d_2d(p.xy), |p.z| − height/2)
/// SDF = min(max(w.x, w.y), 0) + length(max(w, 0))
/// ```
pub struct ExtrudeNode {
    pub profile: Sdf2dNode,
    /// Total height of the solid along Z.
    pub height: f32,
}

impl ExtrudeNode {
    pub fn new(profile: Sdf2dNode, height: f32) -> Self {
        Self { profile, height }
    }
}

impl Sdf for ExtrudeNode {
    fn distance(&self, p: Vec3) -> f32 {
        let d_2d = self.profile.distance(Vec2::new(p.x, p.y));
        let d_z = p.z.abs() - self.height * 0.5;
        let w = Vec2::new(d_2d, d_z);
        w.x.max(w.y).min(0.0) + w.max(Vec2::ZERO).length()
    }
}

// ── RevolveNode ───────────────────────────────────────────────────────────────

/// Which axis to revolve the 2D profile around.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevolveAxis {
    /// Revolve around X: profile sampled at `(sqrt(y²+z²) − offset, x)`.
    X,
    /// Revolve around Y: profile sampled at `(sqrt(x²+z²) − offset, y)`.
    Y,
    /// Revolve around Z (default): profile sampled at `(sqrt(x²+y²) − offset, z)`.
    Z,
}

/// Revolve a 2D profile around a coordinate axis.
///
/// The 2D profile is defined in the half-plane `u ≥ 0` where `u` is the radial
/// distance from the chosen axis and `v` is the position along it.
///
/// # `offset`
///
/// Shifts the profile outward by `offset` model units before revolving.
/// Setting `offset = major_radius` and using a `Circle(minor_radius)` as the
/// profile reproduces `TorusSdf` exactly.
///
/// # Axis conventions (consistent with the rest of the kernel)
///
/// | Axis | Radial plane | Height  | Profile (u, v)               |
/// |------|--------------|---------|------------------------------|
/// | X    | YZ           | X       | `(sqrt(y²+z²) − offset, x)`  |
/// | Y    | XZ           | Y       | `(sqrt(x²+z²) − offset, y)`  |
/// | **Z**| XY           | Z       | `(sqrt(x²+y²) − offset, z)`  |
pub struct RevolveNode {
    pub profile: Sdf2dNode,
    pub axis: RevolveAxis,
    /// Radial offset of the profile from the revolution axis.  Zero = profile
    /// passes through the axis (gives a solid).
    pub offset: f32,
}

impl RevolveNode {
    pub fn around_z(profile: Sdf2dNode, offset: f32) -> Self {
        Self {
            profile,
            axis: RevolveAxis::Z,
            offset,
        }
    }
}

impl Sdf for RevolveNode {
    fn distance(&self, p: Vec3) -> f32 {
        let q = match self.axis {
            RevolveAxis::X => Vec2::new(Vec2::new(p.y, p.z).length() - self.offset, p.x),
            RevolveAxis::Y => Vec2::new(Vec2::new(p.x, p.z).length() - self.offset, p.y),
            RevolveAxis::Z => Vec2::new(Vec2::new(p.x, p.y).length() - self.offset, p.z),
        };
        self.profile.distance(q)
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::primitives::{CubeSdf, CylinderSdf, PrismSdf, TorusSdf};
    use crate::sketch::primitives::{Circle, Rect, RegularPolygon};
    use approx::assert_abs_diff_eq;

    const EPS: f32 = 1e-4;

    // ── ExtrudeNode ────────────────────────────────────────────────────────

    /// Extrude(Circle(r), h) must produce the same SDF as CylinderSdf(r, h).
    #[test]
    fn extrude_circle_equals_cylinder() {
        let extrude = ExtrudeNode::new(Box::new(Circle::new(5.0)), 20.0);
        let cyl = CylinderSdf::new(5.0, 20.0);

        let test_points = [
            Vec3::ZERO,
            Vec3::new(5.0, 0.0, 0.0),  // curved surface
            Vec3::new(0.0, 0.0, 10.0), // top cap
            Vec3::new(8.0, 0.0, 0.0),  // outside radially
            Vec3::new(0.0, 0.0, 14.0), // outside axially
            Vec3::new(8.0, 0.0, 14.0), // outside corner
        ];
        for p in test_points {
            assert_abs_diff_eq!(extrude.distance(p), cyl.distance(p), epsilon = EPS);
        }
    }

    /// Extrude(Rect(w,d), h) must agree with CubeSdf(w, d, h) on key points.
    #[test]
    fn extrude_rect_equals_cube() {
        let extrude = ExtrudeNode::new(Box::new(Rect::new(10.0, 8.0)), 6.0);
        let cube = CubeSdf::new(10.0, 8.0, 6.0);

        let test_points = [
            Vec3::ZERO,
            Vec3::new(5.0, 0.0, 0.0), // +X face
            Vec3::new(0.0, 4.0, 0.0), // +Y face
            Vec3::new(0.0, 0.0, 3.0), // top cap
            Vec3::new(7.0, 0.0, 0.0), // outside +X
            Vec3::new(5.0, 4.0, 3.0), // corner vertex
        ];
        for p in test_points {
            assert_abs_diff_eq!(extrude.distance(p), cube.distance(p), epsilon = EPS);
        }
    }

    /// Extrude(RegularPolygon(n, a), h) must agree with PrismSdf(n, 2a, h).
    #[test]
    fn extrude_hex_polygon_equals_prism() {
        let extrude = ExtrudeNode::new(Box::new(RegularPolygon::new(6, 5.0)), 20.0);
        let prism = PrismSdf::new(6, 10.0, 20.0);

        let test_points = [
            Vec3::ZERO,
            Vec3::new(5.0, 0.0, 0.0),  // face centre
            Vec3::new(0.0, 0.0, 10.0), // top cap
            Vec3::new(8.0, 0.0, 0.0),  // outside radially
        ];
        for p in test_points {
            assert_abs_diff_eq!(extrude.distance(p), prism.distance(p), epsilon = EPS);
        }
    }

    #[test]
    fn extrude_center_is_inside() {
        let e = ExtrudeNode::new(Box::new(Circle::new(5.0)), 20.0);
        assert!(e.distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn extrude_top_cap_on_surface() {
        let e = ExtrudeNode::new(Box::new(Circle::new(5.0)), 20.0);
        assert_abs_diff_eq!(e.distance(Vec3::new(0.0, 0.0, 10.0)), 0.0, epsilon = EPS);
    }

    // ── RevolveNode ────────────────────────────────────────────────────────

    /// Revolve(Circle(minor), Z, major) must reproduce TorusSdf exactly.
    #[test]
    fn revolve_circle_around_z_equals_torus() {
        let major = 8.0_f32;
        let minor = 2.0_f32;
        let revolve = RevolveNode::around_z(Box::new(Circle::new(minor)), major);
        let torus = TorusSdf::new(major, minor);

        let test_points = [
            Vec3::new(major + minor, 0.0, 0.0),       // outer rim +X
            Vec3::new(major - minor, 0.0, 0.0),       // inner rim +X
            Vec3::new(major, 0.0, minor),             // top of tube at 0°
            Vec3::new(0.0, major + minor, 0.0),       // outer rim +Y
            Vec3::ZERO,                               // centre (inside the hole)
            Vec3::new(major + minor + 1.0, 0.0, 0.0), // outside
        ];
        for p in test_points {
            assert_abs_diff_eq!(revolve.distance(p), torus.distance(p), epsilon = EPS);
        }
    }

    #[test]
    fn revolve_rect_around_z_makes_tube() {
        // Revolve a thin horizontal rect (width=2, height=0.5) at offset=6
        // → a flat annular disk (tube cross-section).
        let revolve = RevolveNode::around_z(Box::new(Rect::new(2.0, 0.5)), 6.0);

        // Centre of the rect profile at (6, 0, 0) should be inside.
        assert!(revolve.distance(Vec3::new(6.0, 0.0, 0.0)) < 0.0);
        // Far from axis should be outside.
        assert!(revolve.distance(Vec3::new(20.0, 0.0, 0.0)) > 0.0);
        // Origin (centre of revolution) should be outside (hole in the middle).
        assert!(revolve.distance(Vec3::ZERO) > 0.0);
    }

    #[test]
    fn revolve_zero_offset_with_circle_gives_sphere_like() {
        // Revolve a semicircle (circle centred at 0, no offset):
        // d(p) = |Vec2(r_xy, z)| - radius.  This IS a sphere.
        let revolve = RevolveNode::around_z(Box::new(Circle::new(5.0)), 0.0);
        // At the +X axis: r_xy=5, z=0 → d = |(5,0)| - 5 = 0. On surface. ✓
        assert_abs_diff_eq!(
            revolve.distance(Vec3::new(5.0, 0.0, 0.0)),
            0.0,
            epsilon = EPS
        );
        // At origin: r_xy=0, z=0 → d = |(0,0)| - 5 = -5. ✓
        assert_abs_diff_eq!(revolve.distance(Vec3::ZERO), -5.0, epsilon = EPS);
    }

    #[test]
    fn revolve_axis_x_and_y_are_consistent() {
        // Revolve Circle(r=2) with offset=5 around X resp. Y.
        // The orbital torus surface lies in the plane perpendicular to the axis:
        //   around X → orbit in YZ plane → surface at (x=0, y=7, z=0)
        //   around Y → orbit in XZ plane → surface at (x=7, y=0, z=0)
        let around_x = RevolveNode {
            profile: Box::new(Circle::new(2.0)),
            axis: RevolveAxis::X,
            offset: 5.0,
        };
        let around_y = RevolveNode {
            profile: Box::new(Circle::new(2.0)),
            axis: RevolveAxis::Y,
            offset: 5.0,
        };
        // around_x surface: sqrt(y²+z²)=7, x=0 → (0, 7, 0)
        assert_abs_diff_eq!(
            around_x.distance(Vec3::new(0.0, 7.0, 0.0)),
            0.0,
            epsilon = EPS
        );
        // around_y surface: sqrt(x²+z²)=7, y=0 → (7, 0, 0)
        assert_abs_diff_eq!(
            around_y.distance(Vec3::new(7.0, 0.0, 0.0)),
            0.0,
            epsilon = EPS
        );
    }
}
