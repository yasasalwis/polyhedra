//! Ball bearing SDF — outer race, inner race, and N balls.
use crate::sdf::Sdf;
use glam::Vec3;
use std::f32::consts::TAU;

/// SDF for a ball bearing centred at the origin, axis along Z.
///
/// # Parameters
/// - `outer_radius`: outer race outer radius (mm)
/// - `inner_radius`: inner race inner radius (mm)
/// - `height`: bearing width (mm)
/// - `n_balls`: number of rolling balls
pub struct BearingSdf {
    outer_r: f32,
    inner_r: f32,
    race_w: f32,
    ball_r: f32,
    n_balls: u32,
    half_height: f32,
}

impl BearingSdf {
    /// Create a ball bearing SDF.
    pub fn new(outer_radius: f32, inner_radius: f32, height: f32, n_balls: u32) -> Self {
        let race_w = (outer_radius - inner_radius) * 0.25;
        let ball_r = (outer_radius - inner_radius) * 0.25;
        Self {
            outer_r: outer_radius,
            inner_r: inner_radius,
            race_w,
            ball_r,
            n_balls: n_balls.max(1),
            half_height: height * 0.5,
        }
    }

    /// Signed distance to an annular ring with given centre radius and radial half-width.
    fn annulus_sdf(r_xy: f32, centre_r: f32, half_w: f32, p_z: f32, half_h: f32) -> f32 {
        let d_r = (r_xy - centre_r).abs() - half_w;
        let d_z = p_z.abs() - half_h;
        if d_r > 0.0 || d_z > 0.0 {
            (d_r.max(0.0).powi(2) + d_z.max(0.0).powi(2)).sqrt()
        } else {
            d_r.max(d_z)
        }
    }
}

impl Sdf for BearingSdf {
    fn distance(&self, p: Vec3) -> f32 {
        let r_xy = (p.x * p.x + p.y * p.y).sqrt();

        // Outer race
        let outer_centre_r = self.outer_r - self.race_w * 0.5;
        let d_outer = Self::annulus_sdf(
            r_xy,
            outer_centre_r,
            self.race_w * 0.5,
            p.z,
            self.half_height,
        );

        // Inner race
        let inner_centre_r = self.inner_r + self.race_w * 0.5;
        let d_inner = Self::annulus_sdf(
            r_xy,
            inner_centre_r,
            self.race_w * 0.5,
            p.z,
            self.half_height,
        );

        // Balls: circular arrangement — fold to nearest ball
        let ball_orbit_r = (self.outer_r + self.inner_r) * 0.5;
        let ball_sector = TAU / self.n_balls as f32;
        let theta = p.y.atan2(p.x);
        // Fold angle into [-ball_sector/2, ball_sector/2] to find nearest ball
        let local_theta = {
            let t = theta.rem_euclid(ball_sector);
            if t > ball_sector * 0.5 {
                t - ball_sector
            } else {
                t
            }
        };
        let bx = ball_orbit_r * local_theta.cos();
        let by = ball_orbit_r * local_theta.sin();
        // Distance from p to nearest ball centre (approximated via folded angle)
        let d_ball = ((p.x - bx).powi(2) + (p.y - by).powi(2) + p.z.powi(2)).sqrt() - self.ball_r;

        d_outer.min(d_inner).min(d_ball)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn race_material_inside() {
        let b = BearingSdf::new(30.0, 12.0, 10.0, 8);
        // Midpoint of outer race material
        assert!(b.distance(Vec3::new(28.0, 0.0, 0.0)) < 0.0);
    }

    #[test]
    fn bore_gap_outside() {
        let b = BearingSdf::new(30.0, 12.0, 10.0, 8);
        // Dead centre (inside bore, no material)
        assert!(b.distance(Vec3::ZERO) > 0.0);
    }

    #[test]
    fn far_outside() {
        let b = BearingSdf::new(30.0, 12.0, 10.0, 8);
        assert!(b.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
    }
}
