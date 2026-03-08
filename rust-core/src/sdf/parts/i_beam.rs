//! I-beam / H-beam cross-section SDF, extruded along Z.
use crate::sdf::Sdf;
use glam::Vec3;

/// SDF for an I-beam extruded along Z, centred at the origin.
///
/// # Parameters
/// - `flange_width`: total flange width (mm)
/// - `flange_thickness`: thickness of each flange plate (mm)
/// - `web_height`: clear height between the flanges (mm)
/// - `web_thickness`: thickness of the vertical web (mm)
/// - `length`: total extrusion length along Z (mm)
pub struct IBeamSdf {
    flange_w: f32,
    flange_t: f32,
    web_h: f32,
    web_t: f32,
    half_height: f32,
}

impl IBeamSdf {
    /// Create an I-beam SDF.
    pub fn new(
        flange_width: f32,
        flange_thickness: f32,
        web_height: f32,
        web_thickness: f32,
        length: f32,
    ) -> Self {
        Self {
            flange_w: flange_width,
            flange_t: flange_thickness,
            web_h: web_height,
            web_t: web_thickness,
            half_height: length * 0.5,
        }
    }

    /// Signed distance to an axis-aligned rectangle centred at (cx, cy).
    #[inline]
    fn box2(x: f32, y: f32, cx: f32, cy: f32, hx: f32, hy: f32) -> f32 {
        let dx = (x - cx).abs() - hx;
        let dy = (y - cy).abs() - hy;
        dx.max(dy).min(0.0) + (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt()
    }

    fn d2d(&self, x: f32, y: f32) -> f32 {
        let top_cy = self.web_h * 0.5 + self.flange_t * 0.5;
        let bot_cy = -(self.web_h * 0.5 + self.flange_t * 0.5);

        let d_top = Self::box2(x, y, 0.0, top_cy, self.flange_w * 0.5, self.flange_t * 0.5);
        let d_bot = Self::box2(x, y, 0.0, bot_cy, self.flange_w * 0.5, self.flange_t * 0.5);
        let d_web = Self::box2(x, y, 0.0, 0.0, self.web_t * 0.5, self.web_h * 0.5);

        d_top.min(d_bot).min(d_web)
    }
}

impl Sdf for IBeamSdf {
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
    fn web_center_inside() {
        let b = IBeamSdf::new(50.0, 8.0, 80.0, 6.0, 200.0);
        // Centre of web
        assert!(b.distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn far_outside() {
        let b = IBeamSdf::new(50.0, 8.0, 80.0, 6.0, 200.0);
        assert!(b.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
    }

    #[test]
    fn flange_tip_inside() {
        let b = IBeamSdf::new(50.0, 8.0, 80.0, 6.0, 200.0);
        // Near top flange outer edge: flange centre_y = 40+4 = 44, half_w = 25
        // Point at (24, 44, 0) is within the top flange
        assert!(b.distance(Vec3::new(24.0, 44.0, 0.0)) < 0.0);
    }
}
