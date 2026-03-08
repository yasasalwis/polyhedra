//! Disc cam SDF — eccentric disc extruded along Z.
use crate::sdf::Sdf;
use glam::Vec3;

/// SDF for an eccentric disc cam centred at the origin, axis along Z.
///
/// The disc's geometric centre is offset from the origin by `eccentricity`
/// along the X axis, producing the classic cam lift profile.
///
/// # Parameters
/// - `cam_radius`: disc radius (mm)
/// - `eccentricity`: offset of disc centre from the origin along X (mm)
/// - `height`: disc thickness (mm)
pub struct CamSdf {
    cam_r: f32,
    eccentricity: f32,
    half_height: f32,
}

impl CamSdf {
    /// Create a disc cam SDF.
    pub fn new(cam_radius: f32, eccentricity: f32, height: f32) -> Self {
        Self {
            cam_r: cam_radius,
            eccentricity,
            half_height: height * 0.5,
        }
    }
}

impl Sdf for CamSdf {
    fn distance(&self, p: Vec3) -> f32 {
        // Eccentric disc: circle centred at (eccentricity, 0) in XY
        let dx = p.x - self.eccentricity;
        let r_xy = (dx * dx + p.y * p.y).sqrt();
        let d2d = r_xy - self.cam_r;
        let d_z = p.z.abs() - self.half_height;
        d2d.max(d_z).min(0.0) + (d2d.max(0.0).powi(2) + d_z.max(0.0).powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disc_centre_inside() {
        let c = CamSdf::new(20.0, 8.0, 12.0);
        // At the disc's geometric centre
        assert!(c.distance(Vec3::new(8.0, 0.0, 0.0)) < 0.0);
    }

    #[test]
    fn origin_inside() {
        let c = CamSdf::new(20.0, 8.0, 12.0);
        // Origin should be inside (eccentricity < cam_radius)
        assert!(c.distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn far_outside() {
        let c = CamSdf::new(20.0, 8.0, 12.0);
        assert!(c.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
    }
}
