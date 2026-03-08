//! Gear rack SDF — rectangular bar with periodic spur gear teeth along X.
use crate::sdf::Sdf;
use glam::Vec3;

/// SDF for a gear rack lying along the X axis, centred at the origin.
///
/// Teeth protrude in the +Y direction from the top face of the bar.
///
/// # Parameters
/// - `length`: total bar length along X (mm)
/// - `width`: bar width along Y (mm), excluding teeth
/// - `height`: bar depth along Z (mm)
/// - `tooth_height`: radial tooth height above bar top face (mm)
/// - `pitch`: tooth pitch (mm, centre-to-centre along X)
/// - `tooth_fraction`: fraction of pitch occupied by tooth [0.1, 0.9]
pub struct RackSdf {
    half_length: f32,
    half_width: f32,
    half_height: f32,
    tooth_h: f32,
    pitch: f32,
    tooth_fraction: f32,
}

impl RackSdf {
    /// Create a gear rack SDF.
    pub fn new(
        length: f32,
        width: f32,
        height: f32,
        tooth_height: f32,
        pitch: f32,
        tooth_fraction: f32,
    ) -> Self {
        Self {
            half_length: length * 0.5,
            half_width: width * 0.5,
            half_height: height * 0.5,
            tooth_h: tooth_height,
            pitch,
            tooth_fraction: tooth_fraction.clamp(0.1, 0.9),
        }
    }
}

impl Sdf for RackSdf {
    fn distance(&self, p: Vec3) -> f32 {
        // Bar body
        let dx = p.x.abs() - self.half_length;
        let dy = p.y.abs() - self.half_width;
        let dz = p.z.abs() - self.half_height;
        let d_bar = dx.max(dy).max(dz).min(0.0)
            + (dx.max(0.0).powi(2) + dy.max(0.0).powi(2) + dz.max(0.0).powi(2)).sqrt();

        // Teeth on top face (+Y side), periodic along X
        let x_phase = p.x.rem_euclid(self.pitch) - self.pitch * 0.5; // [-pitch/2, pitch/2]
        let tooth_half_w = self.pitch * self.tooth_fraction * 0.5;
        let tx = x_phase.abs() - tooth_half_w;
        let ty = (p.y - (self.half_width + self.tooth_h * 0.5)).abs() - self.tooth_h * 0.5;
        let tz = p.z.abs() - self.half_height;
        let d_tooth = tx.max(ty).max(tz).min(0.0)
            + (tx.max(0.0).powi(2) + ty.max(0.0).powi(2) + tz.max(0.0).powi(2)).sqrt();

        // Clip teeth to rack length, then union with bar
        if p.x.abs() < self.half_length {
            d_bar.min(d_tooth)
        } else {
            d_bar
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_inside() {
        let r = RackSdf::new(100.0, 20.0, 15.0, 5.0, 8.0, 0.6);
        assert!(r.distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn far_outside() {
        let r = RackSdf::new(100.0, 20.0, 15.0, 5.0, 8.0, 0.6);
        assert!(r.distance(Vec3::new(0.0, 50.0, 0.0)) > 0.0);
    }
}
