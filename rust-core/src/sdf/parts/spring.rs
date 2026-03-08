//! Coil spring SDF — toroidal wire swept along a helix.
use crate::sdf::Sdf;
use glam::Vec3;
use std::f32::consts::TAU;

/// SDF for a coil spring centred at the origin, axis along Z.
///
/// # Parameters
/// - `coil_radius`: distance from the spring axis to the wire centre-line (mm)
/// - `wire_radius`: cross-sectional radius of the wire (mm)
/// - `pitch`: axial distance per turn (mm/turn)
/// - `turns`: number of complete coils
pub struct SpringSdf {
    coil_r: f32,
    wire_r: f32,
    pitch: f32,
    half_height: f32,
}

impl SpringSdf {
    /// Create a coil spring.
    pub fn new(coil_radius: f32, wire_radius: f32, pitch: f32, turns: f32) -> Self {
        Self {
            coil_r: coil_radius,
            wire_r: wire_radius,
            pitch,
            half_height: pitch * turns * 0.5,
        }
    }
}

impl Sdf for SpringSdf {
    fn distance(&self, p: Vec3) -> f32 {
        let k = TAU / self.pitch; // radians per mm along Z

        // Estimate helix turn index from Z, then sample nearby turns
        let t0 = p.z / self.pitch;
        let mut best = f32::MAX;
        for n in -2i32..=2 {
            let turn = (t0 + n as f32).round();
            let hz = (turn * self.pitch).clamp(-self.half_height, self.half_height);
            let angle = hz * k;
            let hpx = self.coil_r * angle.cos();
            let hpy = self.coil_r * angle.sin();
            let hpz = hz;
            let d = ((p.x - hpx).powi(2) + (p.y - hpy).powi(2) + (p.z - hpz).powi(2)).sqrt()
                - self.wire_r;
            if d < best {
                best = d;
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_centre_inside() {
        let s = SpringSdf::new(15.0, 2.0, 8.0, 5.0);
        // Point on wire centre-line at bottom of first coil
        assert!(s.distance(Vec3::new(15.0, 0.0, 0.0)) < 0.0);
    }

    #[test]
    fn far_outside() {
        let s = SpringSdf::new(15.0, 2.0, 8.0, 5.0);
        assert!(s.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
    }
}
