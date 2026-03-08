//! Plus/cross cross-section SDF extruded along Z.
use crate::sdf::Sdf;
use glam::Vec3;

/// SDF for a plus-shaped (cross) prism centred at the origin, extruded along Z.
///
/// # Parameters
/// - `arm_width`: width of each arm (mm)
/// - `arm_length`: full tip-to-tip length of each arm pair (mm)
/// - `height`: extrusion height (mm)
pub struct CrossSectionSdf {
    arm_half_w: f32,
    arm_half_l: f32,
    half_height: f32,
}

impl CrossSectionSdf {
    /// Create a cross/plus SDF.
    pub fn new(arm_width: f32, arm_length: f32, height: f32) -> Self {
        Self {
            arm_half_w: arm_width * 0.5,
            arm_half_l: arm_length * 0.5,
            half_height: height * 0.5,
        }
    }

    #[inline]
    fn box2(x: f32, y: f32, hx: f32, hy: f32) -> f32 {
        let dx = x.abs() - hx;
        let dy = y.abs() - hy;
        dx.max(dy).min(0.0) + (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt()
    }
}

impl Sdf for CrossSectionSdf {
    fn distance(&self, p: Vec3) -> f32 {
        // Union of horizontal arm and vertical arm
        let d_horiz = Self::box2(p.x, p.y, self.arm_half_l, self.arm_half_w);
        let d_vert = Self::box2(p.x, p.y, self.arm_half_w, self.arm_half_l);
        let d2d = d_horiz.min(d_vert);
        let d_z = p.z.abs() - self.half_height;
        d2d.max(d_z).min(0.0) + (d2d.max(0.0).powi(2) + d_z.max(0.0).powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_inside() {
        let c = CrossSectionSdf::new(10.0, 40.0, 10.0);
        assert!(c.distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn arm_tip_inside() {
        let c = CrossSectionSdf::new(10.0, 40.0, 10.0);
        // Near tip of horizontal arm
        assert!(c.distance(Vec3::new(19.0, 0.0, 0.0)) < 0.0);
    }

    #[test]
    fn corner_outside() {
        let c = CrossSectionSdf::new(10.0, 40.0, 10.0);
        // Corner between arms
        assert!(c.distance(Vec3::new(12.0, 12.0, 0.0)) > 0.0);
    }

    #[test]
    fn far_outside() {
        let c = CrossSectionSdf::new(10.0, 40.0, 10.0);
        assert!(c.distance(Vec3::new(50.0, 0.0, 0.0)) > 0.0);
    }
}
