//! Knurl pattern SDF — cylinder with diamond knurl bumps.
use crate::sdf::Sdf;
use glam::Vec3;
use std::f32::consts::TAU;

/// SDF for a knurled cylinder centred at the origin, axis along Z.
///
/// # Parameters
/// - `radius`: base cylinder radius (mm)
/// - `height`: total knurl length (mm)
/// - `bump_depth`: radial height of each knurl bump (mm)
/// - `n_rows`: number of circumferential rows of bumps
/// - `pitch`: axial pitch between bump rows (mm)
pub struct KnurlSdf {
    base_r: f32,
    half_height: f32,
    bump_depth: f32,
    pitch: f32,
    n_rows: u32,
}

impl KnurlSdf {
    /// Create a knurled cylinder.
    pub fn new(radius: f32, height: f32, bump_depth: f32, n_rows: u32, pitch: f32) -> Self {
        Self {
            base_r: radius,
            half_height: height * 0.5,
            bump_depth,
            pitch,
            n_rows,
        }
    }
}

impl Sdf for KnurlSdf {
    fn distance(&self, p: Vec3) -> f32 {
        let r_xy = (p.x * p.x + p.y * p.y).sqrt();
        let theta = p.y.atan2(p.x);

        // Angular period
        let ang_period = TAU / self.n_rows as f32;
        let ang_phase = (theta.rem_euclid(ang_period) / ang_period - 0.5) * 2.0; // [-1, 1]

        // Axial period
        let ax_phase = if self.pitch > 0.0 {
            (p.z.rem_euclid(self.pitch) / self.pitch - 0.5) * 2.0
        } else {
            0.0
        };

        // Diamond bump: amplitude = max(0, 1 - |ang| - |ax|) * depth
        let bump = (1.0 - ang_phase.abs() - ax_phase.abs()).max(0.0) * self.bump_depth;
        let effective_r = self.base_r + bump;

        // Cylinder SDF with bumped radius
        let d_cyl_r = r_xy - effective_r;
        let d_cyl_z = p.z.abs() - self.half_height;
        if d_cyl_r > 0.0 || d_cyl_z > 0.0 {
            (d_cyl_r.max(0.0).powi(2) + d_cyl_z.max(0.0).powi(2)).sqrt()
        } else {
            d_cyl_r.max(d_cyl_z)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_inside() {
        let k = KnurlSdf::new(15.0, 30.0, 0.5, 24, 3.0);
        assert!(k.distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn far_outside() {
        let k = KnurlSdf::new(15.0, 30.0, 0.5, 24, 3.0);
        assert!(k.distance(Vec3::new(50.0, 0.0, 0.0)) > 0.0);
    }
}
