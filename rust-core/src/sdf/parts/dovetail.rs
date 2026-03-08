//! Dovetail slide SDF — trapezoidal cross-section extruded along Z.
use crate::sdf::Sdf;
use glam::Vec3;

/// SDF for a dovetail (trapezoidal) bar extruded along Z, centred at the origin.
///
/// The trapezoid is narrower at the top (y = +half_profile_h) and wider at the
/// bottom (y = -half_profile_h), matching a male dovetail slide.
///
/// # Parameters
/// - `top_width`: width at the narrow (top) face (mm)
/// - `bottom_width`: width at the wide (bottom) face (mm)
/// - `profile_height`: height of the trapezoidal cross-section (mm)
/// - `length`: extrusion length along Z (mm)
pub struct DovetailSdf {
    top_half_w: f32,
    bottom_half_w: f32,
    half_profile_h: f32,
    half_length: f32,
}

impl DovetailSdf {
    /// Create a dovetail bar SDF.
    pub fn new(top_width: f32, bottom_width: f32, profile_height: f32, length: f32) -> Self {
        Self {
            top_half_w: top_width * 0.5,
            bottom_half_w: bottom_width * 0.5,
            half_profile_h: profile_height * 0.5,
            half_length: length * 0.5,
        }
    }

    fn d2d(&self, x: f32, y: f32) -> f32 {
        // Clamp y to profile range for width interpolation
        let t = ((y + self.half_profile_h) / (2.0 * self.half_profile_h)).clamp(0.0, 1.0);
        // t=0 at bottom (wide), t=1 at top (narrow)
        let half_w_at_y = self.bottom_half_w + (self.top_half_w - self.bottom_half_w) * t;

        let dy_top = y - self.half_profile_h; // negative inside top boundary
        let dy_bot = -self.half_profile_h - y; // negative inside bottom boundary
        let dx_r = x - half_w_at_y; // negative inside right wall
        let dx_l = -half_w_at_y - x; // negative inside left wall

        let inside = dy_top < 0.0 && dy_bot < 0.0 && dx_r < 0.0 && dx_l < 0.0;

        if inside {
            // Interior: most positive (least-negative) constraint
            dy_top.max(dy_bot).max(dx_r).max(dx_l)
        } else {
            // Exterior: Euclidean distance to nearest boundary
            let ex = dx_r.max(dx_l).max(0.0);
            let ey = dy_top.max(dy_bot).max(0.0);
            (ex * ex + ey * ey).sqrt()
        }
    }
}

impl Sdf for DovetailSdf {
    fn distance(&self, p: Vec3) -> f32 {
        let d2d = self.d2d(p.x, p.y);
        let d_z = p.z.abs() - self.half_length;
        d2d.max(d_z).min(0.0) + (d2d.max(0.0).powi(2) + d_z.max(0.0).powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_inside() {
        let d = DovetailSdf::new(20.0, 30.0, 15.0, 80.0);
        assert!(d.distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn far_outside() {
        let d = DovetailSdf::new(20.0, 30.0, 15.0, 80.0);
        assert!(d.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
    }

    #[test]
    fn top_corner_outside() {
        let d = DovetailSdf::new(20.0, 30.0, 15.0, 80.0);
        // Past the narrow top edge
        assert!(d.distance(Vec3::new(15.0, 8.0, 0.0)) > 0.0);
    }
}
