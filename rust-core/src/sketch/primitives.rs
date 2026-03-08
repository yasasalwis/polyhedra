//! 2D SDF primitives for the Sketch system.

use std::f32::consts::PI;

use glam::Vec2;

use super::Sdf2d;

// ── Circle ────────────────────────────────────────────────────────────────────

/// Circle centred at the origin.
///
/// ```text
/// d(p) = |p| − radius
/// ```
pub struct Circle {
    pub radius: f32,
}

impl Circle {
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }
}

impl Sdf2d for Circle {
    #[inline]
    fn distance(&self, p: Vec2) -> f32 {
        p.length() - self.radius
    }
}

// ── Rect ──────────────────────────────────────────────────────────────────────

/// Axis-aligned rectangle centred at the origin.
///
/// `width` and `height` are the full extents (not half-extents).
///
/// ```text
/// q = |p| − half
/// d(p) = length(max(q, 0)) + max(q.x, q.y).min(0)
/// ```
pub struct Rect {
    half: Vec2,
}

impl Rect {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            half: Vec2::new(width * 0.5, height * 0.5),
        }
    }

    /// Full width and height.
    pub fn dimensions(&self) -> (f32, f32) {
        (self.half.x * 2.0, self.half.y * 2.0)
    }
}

impl Sdf2d for Rect {
    #[inline]
    fn distance(&self, p: Vec2) -> f32 {
        let q = p.abs() - self.half;
        q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0)
    }
}

// ── RoundedRect ───────────────────────────────────────────────────────────────

/// Rectangle with uniform corner radius.
///
/// The shape is identical to `Rect` offset inward by `radius` then expanded
/// back — equivalent to the Minkowski sum with a disk of the given radius.
pub struct RoundedRect {
    half: Vec2,
    radius: f32,
}

impl RoundedRect {
    /// `width`, `height` — full extents.  `radius` — corner rounding radius.
    ///
    /// # Panics (debug)
    /// If `radius` exceeds `min(width, height) / 2`.
    pub fn new(width: f32, height: f32, radius: f32) -> Self {
        debug_assert!(
            radius <= width.min(height) * 0.5,
            "corner radius {radius} exceeds half of smallest extent"
        );
        Self {
            half: Vec2::new(width * 0.5, height * 0.5),
            radius,
        }
    }
}

impl Sdf2d for RoundedRect {
    #[inline]
    fn distance(&self, p: Vec2) -> f32 {
        let q = p.abs() - self.half + Vec2::splat(self.radius);
        q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - self.radius
    }
}

// ── RegularPolygon ────────────────────────────────────────────────────────────

/// Regular n-sided polygon centred at the origin, exact SDF.
///
/// `apothem` is the inradius — the distance from the centre to the midpoint of
/// a flat face (half of flat-to-flat).  This matches the `PrismSdf` convention
/// so that `ExtrudeNode { RegularPolygon(n, a), h } ≡ PrismSdf(n, 2a, h)`.
///
/// The algorithm is the same n-fold sector fold used in [`PrismSdf`]:
/// fold the angle into the canonical sector `[0, π/n]`, then compute the
/// signed distance to the face segment.
///
/// [`PrismSdf`]: crate::sdf::primitives::PrismSdf
pub struct RegularPolygon {
    pub sides: u32,
    pub apothem: f32,
}

impl RegularPolygon {
    pub fn new(sides: u32, apothem: f32) -> Self {
        Self { sides, apothem }
    }
}

impl Sdf2d for RegularPolygon {
    fn distance(&self, p: Vec2) -> f32 {
        let n = self.sides as f32;
        let a = self.apothem;
        let han = PI / n; // π/n
        let an = 2.0 * han; // 2π/n
        let corner = a * han.tan(); // half-edge width

        let r = p.length();
        let angle = p.y.atan2(p.x);
        let sector_angle = ((angle + han).rem_euclid(an) - han).abs();

        let px = r * sector_angle.cos();
        let py = r * sector_angle.sin();

        let ny = py.clamp(0.0, corner);
        let raw = (px - a).hypot(py - ny);
        if px <= a && py <= corner { -raw } else { raw }
    }
}

// ── Capsule ───────────────────────────────────────────────────────────────────

/// Swept circle (capsule) — the set of points within `radius` of the line
/// segment from `a` to `b`.
///
/// ```text
/// t = clamp(dot(p−a, b−a) / |b−a|², 0, 1)
/// d(p) = |p − a − (b−a)·t| − radius
/// ```
pub struct Capsule {
    pub a: Vec2,
    pub b: Vec2,
    pub radius: f32,
}

impl Capsule {
    /// Horizontal capsule: `a = (−half_length, 0)`, `b = (half_length, 0)`.
    pub fn horizontal(half_length: f32, radius: f32) -> Self {
        Self {
            a: Vec2::new(-half_length, 0.0),
            b: Vec2::new(half_length, 0.0),
            radius,
        }
    }

    /// Vertical capsule: `a = (0, −half_length)`, `b = (0, half_length)`.
    pub fn vertical(half_length: f32, radius: f32) -> Self {
        Self {
            a: Vec2::new(0.0, -half_length),
            b: Vec2::new(0.0, half_length),
            radius,
        }
    }
}

impl Sdf2d for Capsule {
    #[inline]
    fn distance(&self, p: Vec2) -> f32 {
        let ba = self.b - self.a;
        let pa = p - self.a;
        let t = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
        (pa - ba * t).length() - self.radius
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    // ── Circle ─────────────────────────────────────────────────────────────

    #[test]
    fn circle_center_inside() {
        assert!(Circle::new(5.0).distance(Vec2::ZERO) < 0.0);
    }

    #[test]
    fn circle_center_distance() {
        assert_abs_diff_eq!(Circle::new(5.0).distance(Vec2::ZERO), -5.0, epsilon = 1e-6);
    }

    #[test]
    fn circle_surface_is_zero() {
        assert_abs_diff_eq!(
            Circle::new(5.0).distance(Vec2::new(5.0, 0.0)),
            0.0,
            epsilon = 1e-6
        );
    }

    #[test]
    fn circle_outside_distance() {
        assert_abs_diff_eq!(
            Circle::new(5.0).distance(Vec2::new(8.0, 0.0)),
            3.0,
            epsilon = 1e-6
        );
    }

    // ── Rect ───────────────────────────────────────────────────────────────

    #[test]
    fn rect_center_inside() {
        assert!(Rect::new(10.0, 6.0).distance(Vec2::ZERO) < 0.0);
    }

    #[test]
    fn rect_face_is_zero() {
        let r = Rect::new(10.0, 6.0);
        assert_abs_diff_eq!(r.distance(Vec2::new(5.0, 0.0)), 0.0, epsilon = 1e-6);
        assert_abs_diff_eq!(r.distance(Vec2::new(0.0, 3.0)), 0.0, epsilon = 1e-6);
    }

    #[test]
    fn rect_center_distance_equals_negative_min_half() {
        // Nearest face is 3 units away.
        assert_abs_diff_eq!(
            Rect::new(10.0, 6.0).distance(Vec2::ZERO),
            -3.0,
            epsilon = 1e-6
        );
    }

    #[test]
    fn rect_corner_outside() {
        // Corner at (5,3): distance 0.  Point at (6,4): distance sqrt(1+1)≈1.414.
        let r = Rect::new(10.0, 6.0);
        assert_abs_diff_eq!(r.distance(Vec2::new(5.0, 3.0)), 0.0, epsilon = 1e-5);
        assert_abs_diff_eq!(
            r.distance(Vec2::new(6.0, 4.0)),
            2_f32.sqrt(),
            epsilon = 1e-5,
        );
    }

    // ── RoundedRect ────────────────────────────────────────────────────────

    #[test]
    fn rounded_rect_center_inside() {
        assert!(RoundedRect::new(10.0, 6.0, 1.0).distance(Vec2::ZERO) < 0.0);
    }

    #[test]
    fn rounded_rect_face_still_at_half_extent() {
        // The flat face midpoint should still be at x=5, y=0.
        let r = RoundedRect::new(10.0, 6.0, 1.0);
        assert_abs_diff_eq!(r.distance(Vec2::new(5.0, 0.0)), 0.0, epsilon = 1e-5);
    }

    #[test]
    fn rounded_rect_corner_rounded() {
        // The sharp corner at (5,3) moves inward by radius 1 → corner arc centre at (4,2).
        // Point at (5,3) was exactly on corner; now it's outside the arc.
        let sharp = Rect::new(10.0, 6.0);
        let rounded = RoundedRect::new(10.0, 6.0, 1.0);
        // Sharp corner distance = 0; rounded should be > 0 (outside the arc).
        assert_abs_diff_eq!(sharp.distance(Vec2::new(5.0, 3.0)), 0.0, epsilon = 1e-5);
        // The arc centre is at (4,2); point (5,3) is sqrt(2) ≈ 1.414 away.
        // Rounded SDF at (5,3) = sqrt(2) - 1 ≈ 0.414.
        assert_abs_diff_eq!(
            rounded.distance(Vec2::new(5.0, 3.0)),
            2_f32.sqrt() - 1.0,
            epsilon = 1e-5,
        );
    }

    // ── RegularPolygon ─────────────────────────────────────────────────────

    #[test]
    fn hex_center_distance() {
        // apothem=5 → nearest face 5 mm.
        assert_abs_diff_eq!(
            RegularPolygon::new(6, 5.0).distance(Vec2::ZERO),
            -5.0,
            epsilon = 1e-4,
        );
    }

    #[test]
    fn hex_face_centre_is_zero() {
        assert_abs_diff_eq!(
            RegularPolygon::new(6, 5.0).distance(Vec2::new(5.0, 0.0)),
            0.0,
            epsilon = 1e-4,
        );
    }

    #[test]
    fn square_polygon_matches_rect() {
        // n=4, apothem=5 → 10×10 square.  Both should agree at the face centre.
        let sq = RegularPolygon::new(4, 5.0);
        let r = Rect::new(10.0, 10.0);
        assert_abs_diff_eq!(
            sq.distance(Vec2::new(5.0, 0.0)),
            r.distance(Vec2::new(5.0, 0.0)),
            epsilon = 1e-4,
        );
    }

    // ── Capsule ────────────────────────────────────────────────────────────

    #[test]
    fn capsule_midpoint_inside() {
        let c = Capsule::horizontal(3.0, 1.0);
        assert!(c.distance(Vec2::ZERO) < 0.0);
    }

    #[test]
    fn capsule_end_cap_surface() {
        // End cap at x=+3+1=4, y=0.
        let c = Capsule::horizontal(3.0, 1.0);
        assert_abs_diff_eq!(c.distance(Vec2::new(4.0, 0.0)), 0.0, epsilon = 1e-6);
    }

    #[test]
    fn capsule_side_surface() {
        // Side surface at y=1 along the midpoint x=0.
        let c = Capsule::horizontal(3.0, 1.0);
        assert_abs_diff_eq!(c.distance(Vec2::new(0.0, 1.0)), 0.0, epsilon = 1e-6);
    }

    #[test]
    fn capsule_outside_distance() {
        // End-cap centre at (3,0), point at (5,0): dist to segment end = 2, radius = 1 → d=1.
        let c = Capsule::horizontal(3.0, 1.0);
        assert_abs_diff_eq!(c.distance(Vec2::new(5.0, 0.0)), 1.0, epsilon = 1e-6);
    }
}
