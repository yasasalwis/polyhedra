//! Hex bolt SDF — hexagonal head + cylindrical shank.
use crate::sdf::Sdf;
use glam::Vec3;
use std::f32::consts::PI;

/// SDF for a hex bolt, axis along Z, tip at z = 0, head at z = shank_length + head_height.
///
/// # Parameters
/// - `across_flats`: hex head AF (across-flats) dimension (mm)
/// - `head_height`: height of the hexagonal head (mm)
/// - `shank_diameter`: diameter of the threaded shank (mm)
/// - `shank_length`: length of the shank from tip to underside of head (mm)
pub struct HexBoltSdf {
    af: f32,
    head_h: f32,
    shank_r: f32,
    shank_l: f32,
}

impl HexBoltSdf {
    /// Create a hex bolt SDF.
    pub fn new(
        across_flats: f32,
        head_height: f32,
        shank_diameter: f32,
        shank_length: f32,
    ) -> Self {
        Self {
            af: across_flats,
            head_h: head_height,
            shank_r: shank_diameter * 0.5,
            shank_l: shank_length,
        }
    }

    /// 2D SDF for a regular hexagon with given across-flats dimension.
    fn hex_d2d(&self, x: f32, y: f32) -> f32 {
        let apothem = self.af * 0.5; // inradius
        let n = 6u32;
        let an = PI * 2.0 / n as f32; // 60°
        let han = an * 0.5; // 30°

        let r = (x * x + y * y).sqrt();
        let angle = y.atan2(x);

        // Fold angle to canonical [0, 30°) sector
        let sector_angle = ((angle + han).rem_euclid(an) - han).abs();
        // Point on folded sector boundary
        let px = r * sector_angle.cos();
        let py = r * sector_angle.sin();

        // Hex corner half-height (in folded coords)
        let corner = apothem * han.tan();
        let ny = py.clamp(0.0, corner);

        let dx = px - apothem;
        let dy = py - ny;
        let raw = dx.hypot(dy);
        if px <= apothem && py <= corner {
            -raw
        } else {
            raw
        }
    }
}

impl Sdf for HexBoltSdf {
    fn distance(&self, p: Vec3) -> f32 {
        // ── Shank: cylinder from z=0 to z=shank_l ─────────────────────────────
        let d_shank_r = (p.x * p.x + p.y * p.y).sqrt() - self.shank_r;
        let d_shank_z_lo = -p.z; // negative below tip (z<0 outside)
        let d_shank_z_hi = p.z - self.shank_l; // negative above shank top
        let d_shank = if d_shank_r > 0.0 || d_shank_z_lo > 0.0 || d_shank_z_hi > 0.0 {
            let er = d_shank_r.max(0.0);
            let ez = d_shank_z_lo.max(d_shank_z_hi).max(0.0);
            (er * er + ez * ez).sqrt()
        } else {
            d_shank_r.max(d_shank_z_lo).max(d_shank_z_hi)
        };

        // ── Head: hex prism from z=shank_l to z=shank_l+head_h ───────────────
        let d_head_hex = self.hex_d2d(p.x, p.y);
        let d_head_z_lo = -(p.z - self.shank_l); // negative above shank top
        let d_head_z_hi = p.z - (self.shank_l + self.head_h); // negative below head top
        let d_head = if d_head_hex > 0.0 || d_head_z_lo > 0.0 || d_head_z_hi > 0.0 {
            let er = d_head_hex.max(0.0);
            let ez = d_head_z_lo.max(d_head_z_hi).max(0.0);
            (er * er + ez * ez).sqrt()
        } else {
            d_head_hex.max(d_head_z_lo).max(d_head_z_hi)
        };

        d_shank.min(d_head)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shank_mid_inside() {
        let b = HexBoltSdf::new(17.0, 10.0, 10.0, 30.0);
        assert!(b.distance(Vec3::new(0.0, 0.0, 15.0)) < 0.0);
    }

    #[test]
    fn head_mid_inside() {
        let b = HexBoltSdf::new(17.0, 10.0, 10.0, 30.0);
        // Centre of head
        assert!(b.distance(Vec3::new(0.0, 0.0, 35.0)) < 0.0);
    }

    #[test]
    fn far_outside() {
        let b = HexBoltSdf::new(17.0, 10.0, 10.0, 30.0);
        assert!(b.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
    }

    #[test]
    fn below_tip_outside() {
        let b = HexBoltSdf::new(17.0, 10.0, 10.0, 30.0);
        assert!(b.distance(Vec3::new(0.0, 0.0, -5.0)) > 0.0);
    }
}
