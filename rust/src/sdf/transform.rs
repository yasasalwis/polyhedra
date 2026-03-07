//! Domain-transformation wrappers for SDF nodes.
//!
//! Rather than moving geometry, we transform the *query point* into the
//! SDF's local space. This keeps SDF functions simple and composable.

use crate::math::{Quat, Transform, Vec3};
use crate::sdf::{Sdf, SdfNode};

// ── Translation ───────────────────────────────────────────────────────────

/// Move an SDF to a new position.
pub struct Translate {
    pub inner:  SdfNode,
    pub offset: Vec3,
}
impl Sdf for Translate {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        self.inner.distance(p - self.offset)
    }
}

// ── Uniform scale ─────────────────────────────────────────────────────────

/// Scale an SDF uniformly. Non-uniform scaling is handled by `ScaleNonUniform`.
pub struct Scale {
    pub inner:  SdfNode,
    pub factor: f32,
}
impl Sdf for Scale {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        // Divide query point by scale factor, then scale the result back.
        self.inner.distance(p / self.factor) * self.factor
    }
}

// ── Non-uniform scale ─────────────────────────────────────────────────────

/// Scale an SDF independently along X, Y, Z.
///
/// Note: Non-uniform scaling slightly distorts distance fields near surfaces.
/// For visual quality this is acceptable; for precise measurements use with care.
pub struct ScaleNonUniform {
    pub inner: SdfNode,
    pub scale: Vec3,
}
impl Sdf for ScaleNonUniform {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        let min_scale = self.scale.x.min(self.scale.y).min(self.scale.z);
        self.inner.distance(p / self.scale) * min_scale
    }
}

// ── Rotation ─────────────────────────────────────────────────────────────

/// Rotate an SDF by a quaternion. The inverse rotation is applied to the
/// query point to keep the SDF in local space.
pub struct Rotate {
    pub inner:       SdfNode,
    pub rotation:    Quat,
    pub inv_rotation: Quat,
}
impl Rotate {
    pub fn new(inner: SdfNode, rotation: Quat) -> Self {
        Self { inner, rotation, inv_rotation: rotation.inverse() }
    }
}
impl Sdf for Rotate {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        self.inner.distance(self.inv_rotation * p)
    }
}

// ── Mirror ────────────────────────────────────────────────────────────────

/// Mirror an SDF across an axis-aligned plane.
pub enum MirrorPlane {
    Yz, // mirror across YZ (flip X)
    Xz, // mirror across XZ (flip Y)
    Xy, // mirror across XY (flip Z)
}

pub struct Mirror {
    pub inner: SdfNode,
    pub plane: MirrorPlane,
}
impl Sdf for Mirror {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        let mirrored = match self.plane {
            MirrorPlane::Yz => Vec3::new(p.x.abs(), p.y, p.z),
            MirrorPlane::Xz => Vec3::new(p.x, p.y.abs(), p.z),
            MirrorPlane::Xy => Vec3::new(p.x, p.y, p.z.abs()),
        };
        self.inner.distance(mirrored)
    }
}

// ── General matrix transform ──────────────────────────────────────────────

/// Apply an arbitrary `Transform` (translation + rotation + scale) to an SDF.
pub struct TransformNode {
    pub inner:     SdfNode,
    pub transform: Transform,
}
impl Sdf for TransformNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        self.inner.distance(self.transform.inverse_transform_point(p))
    }
}
