//! Countersunk hole SDF — conical recess + cylindrical bore.
//!
//! Returns a *negative* SDF (interior = inside the hole).
//! Use with a boolean difference to cut it into a solid.
use crate::sdf::Sdf;
use glam::Vec3;

/// SDF for a countersunk hole, axis along -Z, opening at z = 0.
///
/// Convention: z = 0 is the material surface; z < 0 is into the material.
/// The SDF is negative inside the hole and positive outside.
///
/// # Parameters
/// - `bore_diameter`: diameter of the cylindrical bore (mm)
/// - `csk_diameter`: outer diameter of the countersink cone at the surface (mm)
/// - `csk_depth`: axial depth of the conical countersink (mm)
/// - `total_depth`: total axial depth from surface through bore (mm)
pub struct CskHoleSdf {
    bore_r: f32,
    csk_r: f32,
    csk_depth: f32,
    total_depth: f32,
}

impl CskHoleSdf {
    /// Create a countersunk hole SDF.
    pub fn new(bore_diameter: f32, csk_diameter: f32, csk_depth: f32, total_depth: f32) -> Self {
        Self {
            bore_r: bore_diameter * 0.5,
            csk_r: csk_diameter * 0.5,
            csk_depth,
            total_depth,
        }
    }
}

impl Sdf for CskHoleSdf {
    fn distance(&self, p: Vec3) -> f32 {
        let r_xy = (p.x * p.x + p.y * p.y).sqrt();
        // z_depth: positive = into material (below surface)
        let z_depth = -p.z;

        // Effective radius at this depth: linear taper from csk_r to bore_r over csk_depth
        let cone_r = if z_depth <= 0.0 {
            // Above surface: use csk_r so the cone clips cleanly
            self.csk_r
        } else if z_depth < self.csk_depth {
            self.csk_r + (self.bore_r - self.csk_r) * (z_depth / self.csk_depth)
        } else {
            self.bore_r
        };

        // Inside radially: r_xy < cone_r  → d_r < 0
        let d_r = r_xy - cone_r;
        // Inside axially (between surface and bottom): → d_z_top < 0 and d_z_bot < 0
        let d_z_top = -z_depth; // negative below surface
        let d_z_bot = z_depth - self.total_depth; // negative above bore bottom

        // Interior: all three constraints negative
        // SDF = max(d_r, d_z_top, d_z_bot)
        d_r.max(d_z_top).max(d_z_bot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bore_centre_inside() {
        let h = CskHoleSdf::new(6.0, 12.0, 4.0, 20.0);
        // Deep in bore, on axis
        assert!(h.distance(Vec3::new(0.0, 0.0, -10.0)) < 0.0);
    }

    #[test]
    fn above_surface_outside() {
        let h = CskHoleSdf::new(6.0, 12.0, 4.0, 20.0);
        // Above surface
        assert!(h.distance(Vec3::new(0.0, 0.0, 5.0)) > 0.0);
    }

    #[test]
    fn below_bore_bottom_outside() {
        let h = CskHoleSdf::new(6.0, 12.0, 4.0, 20.0);
        // Below total depth
        assert!(h.distance(Vec3::new(0.0, 0.0, -25.0)) > 0.0);
    }
}
