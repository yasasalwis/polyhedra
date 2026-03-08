//! Signed Distance Function (SDF) engine.
//!
//! Every shape in polyhedra is a function `f(p) → f32` where:
//!   - `f(p) < 0`  → point `p` is **inside** the shape
//!   - `f(p) = 0`  → point `p` is **on the surface**
//!   - `f(p) > 0`  → point `p` is **outside** the shape
//!
//! Boolean operations (union, difference, intersection) and smooth blending
//! (fillets, chamfers) are all expressed as simple mathematical operations on
//! the distance values — no mesh manipulation required.

pub mod operations;
pub mod parts;
pub mod primitives;
pub mod transform;

use crate::math::Vec3;

// ── Core SDF trait ────────────────────────────────────────────────────────

/// Trait implemented by every shape and operation node in the SDF tree.
pub trait Sdf: Send + Sync {
    /// Evaluate the signed distance from point `p` to the nearest surface.
    fn distance(&self, p: Vec3) -> f32;

    /// Compute the surface normal at point `p` via central finite differences.
    ///
    /// The normal points outward (away from the interior).
    /// Override this in shapes that have an analytical gradient for speed.
    fn normal(&self, p: Vec3) -> Vec3 {
        const EPS: f32 = 1e-4;
        let dx = self.distance(p + Vec3::X * EPS) - self.distance(p - Vec3::X * EPS);
        let dy = self.distance(p + Vec3::Y * EPS) - self.distance(p - Vec3::Y * EPS);
        let dz = self.distance(p + Vec3::Z * EPS) - self.distance(p - Vec3::Z * EPS);
        Vec3::new(dx, dy, dz).normalize_or_zero()
    }
}

// ── Boxed SDF alias ───────────────────────────────────────────────────────

/// A heap-allocated, type-erased SDF node.
/// The SDF tree is composed entirely of `SdfNode` values.
pub type SdfNode = Box<dyn Sdf>;

// ── Utility: wrap an SdfNode in a transform ───────────────────────────────

use crate::math::Transform;

/// An SDF node with an applied spatial transform.
/// Transforms a world-space query point into local space before evaluating.
pub struct TransformedSdf {
    pub inner: SdfNode,
    pub transform: Transform,
}

impl Sdf for TransformedSdf {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        // Move the query point into the SDF's local coordinate system.
        let local = self.transform.inverse_transform_point(p);
        self.inner.distance(local)
    }
}
