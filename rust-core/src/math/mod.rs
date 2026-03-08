//! Linear algebra types re-exported from `glam`.
//!
//! All geometry in polyhedra_core is expressed in millimetres (f32).
//! Phase 1 will add Ray, Transform, and higher-level helpers.

pub use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

// ── Ray ───────────────────────────────────────────────────────────────────

/// A ray with an origin and a normalised direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3, // must be normalised
}

impl Ray {
    #[inline]
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Point on the ray at parameter `t`.
    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

// ── Transform ─────────────────────────────────────────────────────────────

/// A rigid-body + scale transform stored as a 4×4 matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub matrix: Mat4,
    pub inverse: Mat4,
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            matrix: Mat4::IDENTITY,
            inverse: Mat4::IDENTITY,
        }
    }

    pub fn from_translation(t: Vec3) -> Self {
        let m = Mat4::from_translation(t);
        Self {
            matrix: m,
            inverse: m.inverse(),
        }
    }

    pub fn from_rotation(q: Quat) -> Self {
        let m = Mat4::from_quat(q);
        Self {
            matrix: m,
            inverse: m.inverse(),
        }
    }

    pub fn from_scale(s: Vec3) -> Self {
        let m = Mat4::from_scale(s);
        Self {
            matrix: m,
            inverse: m.inverse(),
        }
    }

    /// Apply this transform to a world-space point (forward).
    #[inline]
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        self.matrix.transform_point3(p)
    }

    /// Apply the inverse transform to move a point into local space.
    #[inline]
    pub fn inverse_transform_point(&self, p: Vec3) -> Vec3 {
        self.inverse.transform_point3(p)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}
