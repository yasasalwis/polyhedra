//! Chain sprocket SDF — disc with triangular teeth around the perimeter.
use crate::sdf::Sdf;
use glam::Vec3;
use std::f32::consts::TAU;

/// SDF for a chain sprocket centred at the origin, axis along Z.
///
/// # Parameters
/// - `pitch_radius`: base circle radius (mm)
/// - `tooth_height`: radial tooth height above base circle (mm)
/// - `bore_radius`: central bore hole radius (mm)
/// - `height`: sprocket thickness (mm)
/// - `n_teeth`: number of sprocket teeth
pub struct SprocketSdf {
    pitch_r: f32,
    tooth_h: f32,
    bore_r: f32,
    half_height: f32,
    n_teeth: u32,
}

impl SprocketSdf {
    /// Create a chain sprocket SDF.
    pub fn new(
        pitch_radius: f32,
        tooth_height: f32,
        bore_radius: f32,
        height: f32,
        n_teeth: u32,
    ) -> Self {
        Self {
            pitch_r: pitch_radius,
            tooth_h: tooth_height,
            bore_r: bore_radius,
            half_height: height * 0.5,
            n_teeth: n_teeth.max(3),
        }
    }
}

impl Sdf for SprocketSdf {
    fn distance(&self, p: Vec3) -> f32 {
        let n = self.n_teeth as f32;
        let sector = TAU / n;
        let han = sector * 0.5; // half sector angle

        let r_xy = (p.x * p.x + p.y * p.y).sqrt();
        let theta = p.y.atan2(p.x);

        // Fold into canonical sector; 0 = tooth tip, han = valley
        let local_theta = theta.rem_euclid(sector);
        let offset = (local_theta - han).abs();

        // Triangular tooth profile
        let frac = (offset / han).min(1.0);
        let effective_r = (self.pitch_r + self.tooth_h) - frac * self.tooth_h;

        // 2D profile: outer shape minus bore
        let d2d_outer = r_xy - effective_r;
        let d_bore = self.bore_r - r_xy; // positive inside bore
        let d2d = d2d_outer.max(d_bore);

        // Extrude along Z
        let d_z = p.z.abs() - self.half_height;
        d2d.max(d_z).min(0.0) + (d2d.max(0.0).powi(2) + d_z.max(0.0).powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mid_disc_inside() {
        let s = SprocketSdf::new(30.0, 5.0, 8.0, 10.0, 16);
        // Midway between bore and pitch circle
        assert!(s.distance(Vec3::new(20.0, 0.0, 0.0)) < 0.0);
    }

    #[test]
    fn bore_centre_outside() {
        let s = SprocketSdf::new(30.0, 5.0, 8.0, 10.0, 16);
        // Dead centre is inside the bore, so SDF should be positive
        assert!(s.distance(Vec3::ZERO) > 0.0);
    }

    #[test]
    fn far_outside() {
        let s = SprocketSdf::new(30.0, 5.0, 8.0, 10.0, 16);
        assert!(s.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
    }
}
