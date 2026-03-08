//! Metric threaded rod SDF — cylinder with helical V-groove.
use crate::sdf::Sdf;
use glam::Vec3;
use std::f32::consts::{PI, TAU};

/// SDF for a metric-style threaded rod centred at the origin, axis along Z.
///
/// # Parameters
/// - `outer_radius`: major (crest) radius in mm
/// - `pitch`: axial distance between thread crests in mm/turn
/// - `height`: total rod length in mm
pub struct ThreadSdf {
    outer_r: f32,
    pitch: f32,
    half_height: f32,
    depth: f32,
}

impl ThreadSdf {
    /// Create a metric threaded rod.
    pub fn new(outer_radius: f32, pitch: f32, height: f32) -> Self {
        // ISO metric thread depth ≈ 0.6495 * pitch
        let depth = 0.6495 * pitch;
        Self {
            outer_r: outer_radius,
            pitch,
            half_height: height * 0.5,
            depth,
        }
    }
}

impl Sdf for ThreadSdf {
    fn distance(&self, p: Vec3) -> f32 {
        // Outer cylinder SDF
        let r_xy = (p.x * p.x + p.y * p.y).sqrt();
        let d_cyl_r = r_xy - self.outer_r;
        let d_cyl_z = p.z.abs() - self.half_height;
        let d_cyl = if d_cyl_r > 0.0 || d_cyl_z > 0.0 {
            (d_cyl_r.max(0.0).powi(2) + d_cyl_z.max(0.0).powi(2)).sqrt()
        } else {
            d_cyl_r.max(d_cyl_z)
        };

        // Only apply groove subtraction when near the cylinder surface
        // (further than half the cylinder radius from centre = close to surface region)
        if r_xy < self.outer_r * 0.5 {
            // Well inside the core: just return cylinder distance (no groove effect)
            return d_cyl;
        }

        // Helical phase mapped to [-PI, PI)
        let theta = p.y.atan2(p.x);
        let k = TAU / self.pitch; // radians per mm along Z
        let mut phase = (theta - k * p.z).rem_euclid(TAU);
        if phase > PI {
            phase -= TAU;
        }
        // frac in [-1, 1]; 0 = thread crest
        let frac = phase / PI;

        // V-groove: signed distance in (radial, axial-groove) space
        // dr measured from mid-depth of groove
        let dr = r_xy - (self.outer_r - self.depth * 0.5);
        let axial = frac * self.depth;
        // dv < 0 means inside groove
        let dv = dr.abs() + axial.abs() - self.depth * 0.5;

        // Thread = cylinder minus groove interior
        d_cyl.max(dv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_inside() {
        let t = ThreadSdf::new(10.0, 2.0, 30.0);
        assert!(t.distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn far_outside() {
        let t = ThreadSdf::new(10.0, 2.0, 30.0);
        assert!(t.distance(Vec3::new(50.0, 0.0, 0.0)) > 0.0);
    }

    #[test]
    fn above_end_outside() {
        let t = ThreadSdf::new(10.0, 2.0, 30.0);
        assert!(t.distance(Vec3::new(0.0, 0.0, 20.0)) > 0.0);
    }
}
