//! Splined shaft SDF — cylinder with external longitudinal spline teeth.
use crate::sdf::Sdf;
use glam::Vec3;
use std::f32::consts::TAU;

/// SDF for a splined (involute-approximated) shaft centred at the origin, axis along Z.
///
/// # Parameters
/// - `pitch_radius`: base circle radius (mm)
/// - `tooth_height`: radial height of each spline tooth (mm)
/// - `height`: shaft length (mm)
/// - `n_splines`: number of spline teeth
/// - `tooth_fraction`: fraction of each angular sector occupied by the tooth [0.1, 0.9]
pub struct SplineSdf {
    pitch_r: f32,
    tooth_height: f32,
    half_height: f32,
    n_splines: u32,
    tooth_fraction: f32,
}

impl SplineSdf {
    /// Create a splined shaft.
    pub fn new(
        pitch_radius: f32,
        tooth_height: f32,
        height: f32,
        n_splines: u32,
        tooth_fraction: f32,
    ) -> Self {
        Self {
            pitch_r: pitch_radius,
            tooth_height,
            half_height: height * 0.5,
            n_splines,
            tooth_fraction: tooth_fraction.clamp(0.1, 0.9),
        }
    }
}

impl Sdf for SplineSdf {
    fn distance(&self, p: Vec3) -> f32 {
        let n = self.n_splines as f32;
        let sector = TAU / n;
        let han = sector * 0.5; // half sector angle

        let r_xy = (p.x * p.x + p.y * p.y).sqrt();
        let theta = p.y.atan2(p.x);

        // Fold into canonical sector [0, sector]; measure offset from bisector
        let local_theta = theta.rem_euclid(sector);
        let offset = (local_theta - han).abs(); // 0 = tooth tip, han = valley

        // Tooth tip at offset = 0, valley at offset = han
        let tooth_half_ang = han * self.tooth_fraction;
        let frac = (offset / tooth_half_ang).min(1.0);
        let tip_r = self.pitch_r + self.tooth_height;
        let effective_r = tip_r - frac * self.tooth_height;

        // 2D signed distance to the effective circle at this angle
        let d2d = r_xy - effective_r;

        // Extrude along Z (Quilez method)
        let d_z = p.z.abs() - self.half_height;
        d2d.max(d_z).min(0.0) + (d2d.max(0.0).powi(2) + d_z.max(0.0).powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_inside() {
        let s = SplineSdf::new(15.0, 3.0, 40.0, 12, 0.5);
        assert!(s.distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn far_outside() {
        let s = SplineSdf::new(15.0, 3.0, 40.0, 12, 0.5);
        assert!(s.distance(Vec3::new(50.0, 0.0, 0.0)) > 0.0);
    }
}
