//! T-slot extrusion SDF — square bar with T-shaped slots on all four faces.
use crate::sdf::Sdf;
use glam::Vec3;

/// SDF for a T-slot aluminium extrusion profile centred at the origin, extruded along Z.
///
/// # Parameters
/// - `side`: square cross-section side length (mm)
/// - `slot_width`: narrow mouth opening of each T-slot (mm)
/// - `slot_head_width`: wider inner recess of each T-slot (mm)
/// - `slot_depth`: total axial depth of each T-slot from face (mm)
/// - `length`: bar length along Z (mm)
pub struct TSlotSdf {
    half_side: f32,
    slot_w: f32,  // half of slot mouth width
    slot_hw: f32, // half of slot head width
    slot_depth: f32,
    half_height: f32,
}

impl TSlotSdf {
    /// Create a T-slot extrusion SDF.
    pub fn new(
        side: f32,
        slot_width: f32,
        slot_head_width: f32,
        slot_depth: f32,
        length: f32,
    ) -> Self {
        Self {
            half_side: side * 0.5,
            slot_w: slot_width * 0.5,
            slot_hw: slot_head_width * 0.5,
            slot_depth,
            half_height: length * 0.5,
        }
    }

    #[inline]
    fn box2(x: f32, y: f32, hx: f32, hy: f32) -> f32 {
        let dx = x.abs() - hx;
        let dy = y.abs() - hy;
        dx.max(dy).min(0.0) + (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt()
    }

    /// T-slot groove opening toward +Y face, centred at (0, half_side).
    fn t_slot_groove_y(&self, x: f32, y: f32) -> f32 {
        let mouth_depth = self.slot_depth * 0.35;
        let head_depth = self.slot_depth * 0.65;
        let face = self.half_side;

        // Mouth: narrow channel near surface
        let mouth_cy = face - mouth_depth * 0.5;
        let d_mouth = Self::box2(x, y - mouth_cy, self.slot_w, mouth_depth * 0.5);

        // Head: wider recess deeper inside
        let head_cy = face - mouth_depth - head_depth * 0.5;
        let d_head = Self::box2(x, y - head_cy, self.slot_hw, head_depth * 0.5);

        d_mouth.min(d_head)
    }
}

impl Sdf for TSlotSdf {
    fn distance(&self, p: Vec3) -> f32 {
        // Base square bar
        let d_bar = Self::box2(p.x, p.y, self.half_side, self.half_side);

        // Four T-slot grooves, one per face
        let dg_py = self.t_slot_groove_y(p.x, p.y); // +Y face
        let dg_ny = self.t_slot_groove_y(p.x, -p.y); // -Y face
        let dg_px = self.t_slot_groove_y(p.y, p.x); // +X face
        let dg_nx = self.t_slot_groove_y(p.y, -p.x); // -X face

        let d_slots = dg_py.min(dg_ny).min(dg_px).min(dg_nx);

        // Bar minus grooves: intersection of bar with complement of slot interiors
        let d2d = d_bar.max(-d_slots);

        // Extrude along Z
        let d_z = p.z.abs() - self.half_height;
        d2d.max(d_z).min(0.0) + (d2d.max(0.0).powi(2) + d_z.max(0.0).powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_inside() {
        let t = TSlotSdf::new(40.0, 8.0, 16.0, 8.0, 100.0);
        assert!(t.distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn far_outside() {
        let t = TSlotSdf::new(40.0, 8.0, 16.0, 8.0, 100.0);
        assert!(t.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
    }
}
