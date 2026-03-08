//! Spur gear SDF.
//!
//! A cylindrical gear with N evenly-spaced rectangular teeth around its
//! perimeter.  The teeth are centred within each angular sector.
//!
//! # Parameters
//! - `teeth`:          number of teeth (integer ≥ 3)
//! - `pitch_radius`:   radius at the midpoint of the tooth height (mm)
//! - `tooth_height`:   total radial extent of each tooth (tip to root) (mm)
//! - `tooth_fraction`: fraction of each sector occupied by a tooth [0.1, 0.9]
//! - `half_height`:    half of the gear disk height along Z (mm)

use std::f32::consts::TAU;

use glam::{Vec2, Vec3};

use crate::sdf::Sdf;

/// SDF for a spur gear centred at the origin, axis along Z.
pub struct GearSdf {
    teeth: u32,
    pitch_radius: f32,
    tooth_height: f32,
    tooth_fraction: f32,
    half_height: f32,
}

impl GearSdf {
    /// Create a new gear SDF.
    ///
    /// # Parameters
    /// - `teeth`:          number of teeth (≥ 3)
    /// - `pitch_radius`:   mid-tooth radius (mm)
    /// - `tooth_height`:   full tooth depth tip-to-root (mm)
    /// - `tooth_fraction`: fraction of sector width used by the tooth [0.1, 0.9]
    /// - `height`:         overall gear disk height (mm)
    pub fn new(
        teeth: u32,
        pitch_radius: f32,
        tooth_height: f32,
        tooth_fraction: f32,
        height: f32,
    ) -> Self {
        Self {
            teeth: teeth.max(3),
            pitch_radius,
            tooth_height,
            tooth_fraction: tooth_fraction.clamp(0.1, 0.9),
            half_height: height * 0.5,
        }
    }

    /// Outer radius (tooth tip circle).
    pub fn tip_radius(&self) -> f32 {
        self.pitch_radius + self.tooth_height * 0.5
    }
}

impl Sdf for GearSdf {
    fn distance(&self, p: Vec3) -> f32 {
        let n = self.teeth as f32;
        let sector = TAU / n; // angular width of one tooth period

        // Half the angular arc used by a tooth on each side of the sector centre.
        let half_tooth_arc = sector * 0.5 * self.tooth_fraction;

        // 2D polar coordinates.
        let r = Vec2::new(p.x, p.y).length();
        let angle = p.y.atan2(p.x).rem_euclid(sector); // angle in [0, sector)

        // Angular distance from the sector centre (sector/2).
        let dist_from_center = (angle - sector * 0.5).abs();

        // Choose the boundary radius based on tooth vs. gap.
        let r_tip = self.pitch_radius + self.tooth_height * 0.5;
        let r_root = self.pitch_radius - self.tooth_height * 0.5;
        let r_boundary = if dist_from_center < half_tooth_arc {
            r_tip // inside a tooth
        } else {
            r_root // inside a gap
        };

        // 2D signed distance to the gear profile.
        let d_2d = r - r_boundary;

        // Exact extrusion along Z (Quilez method).
        let d_z = p.z.abs() - self.half_height;
        let w = Vec2::new(d_2d, d_z);
        w.x.max(w.y).min(0.0) + w.max(Vec2::ZERO).length()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gear12() -> GearSdf {
        GearSdf::new(12, 25.0, 5.0, 0.5, 12.0)
    }

    #[test]
    fn center_inside() {
        assert!(gear12().distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn far_outside() {
        assert!(gear12().distance(Vec3::new(50.0, 0.0, 0.0)) > 0.0);
    }

    #[test]
    fn above_top_outside() {
        assert!(gear12().distance(Vec3::new(0.0, 0.0, 20.0)) > 0.0);
    }
}
