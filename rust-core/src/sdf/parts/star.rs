//! N-pointed star SDF extruded along Z.
use crate::sdf::Sdf;
use glam::Vec3;
use std::f32::consts::TAU;

/// SDF for an N-pointed star prism centred at the origin, extruded along Z.
///
/// # Parameters
/// - `outer_radius`: tip radius (mm)
/// - `inner_radius`: valley radius (mm)
/// - `n_points`: number of star points (≥ 3)
/// - `height`: extrusion height (mm)
pub struct StarSdf {
    outer_r: f32,
    inner_r: f32,
    n_points: u32,
    half_height: f32,
}

impl StarSdf {
    /// Create an N-pointed star SDF.
    pub fn new(outer_radius: f32, inner_radius: f32, points: u32, height: f32) -> Self {
        Self {
            outer_r: outer_radius,
            inner_r: inner_radius,
            n_points: points.max(3),
            half_height: height * 0.5,
        }
    }

    fn d2d(&self, x: f32, y: f32) -> f32 {
        let n = self.n_points as f32;
        let sector = TAU / n;
        let r_xy = (x * x + y * y).sqrt();
        let theta = y.atan2(x);

        // Fold into one sector [0, sector], then measure distance from bisector
        let local_theta = theta.rem_euclid(sector);
        let half_sector = sector * 0.5;
        // t=0 at valley (bisector centre), t=1 at tip
        let t = (local_theta - half_sector).abs() / half_sector;
        let effective_r = self.inner_r + (self.outer_r - self.inner_r) * t;

        r_xy - effective_r
    }
}

impl Sdf for StarSdf {
    fn distance(&self, p: Vec3) -> f32 {
        let d2d = self.d2d(p.x, p.y);
        let d_z = p.z.abs() - self.half_height;
        d2d.max(d_z).min(0.0) + (d2d.max(0.0).powi(2) + d_z.max(0.0).powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_inside() {
        let s = StarSdf::new(20.0, 10.0, 5, 10.0);
        // Inner circle should be fully inside
        assert!(s.distance(Vec3::new(5.0, 0.0, 0.0)) < 0.0);
    }

    #[test]
    fn far_outside() {
        let s = StarSdf::new(20.0, 10.0, 5, 10.0);
        assert!(s.distance(Vec3::new(50.0, 0.0, 0.0)) > 0.0);
    }

    #[test]
    fn above_top_outside() {
        let s = StarSdf::new(20.0, 10.0, 5, 10.0);
        assert!(s.distance(Vec3::new(0.0, 0.0, 10.0)) > 0.0);
    }
}
