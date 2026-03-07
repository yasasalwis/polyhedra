//! SDF boolean operations and smooth blending functions.
//!
//! These are the core mathematical operations that make SDF-based geometry
//! so elegant. Boolean operations on meshes require complex algorithms;
//! here they reduce to two lines of arithmetic.

use crate::math::Vec3;
use crate::sdf::{Sdf, SdfNode};

// ── Boolean operations (exact) ────────────────────────────────────────────

/// Union: the region covered by A **or** B.
///
/// Equivalent to: keep the part closest to any surface.
#[inline]
pub fn union(a: f32, b: f32) -> f32 {
    a.min(b)
}

/// Difference: the region inside A **but not** B (A minus B).
///
/// Equivalent to: cut B out of A.
#[inline]
pub fn difference(a: f32, b: f32) -> f32 {
    a.max(-b)
}

/// Intersection: the region inside **both** A and B.
#[inline]
pub fn intersection(a: f32, b: f32) -> f32 {
    a.max(b)
}

// ── Smooth blending (fillets and chamfers) ────────────────────────────────

/// Smooth minimum — creates a **rounded fillet** between two SDF surfaces.
///
/// `k` is the blend radius in the same units as the model (millimetres).
/// When `k = 0`, this degenerates to the exact `union`.
///
/// This is the key operation that gives polyhedra perfect fillets natively —
/// no mesh post-processing needed.
#[inline]
pub fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return union(a, b);
    }
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b.mul_add(1.0 - h, a * h) - k * h * (1.0 - h)
}

/// Smooth maximum — the dual of `smooth_min`, used for rounded intersections.
#[inline]
pub fn smooth_max(a: f32, b: f32, k: f32) -> f32 {
    -smooth_min(-a, -b, k)
}

/// Smooth difference — rounded cutout (chamfered removal of B from A).
#[inline]
pub fn smooth_difference(a: f32, b: f32, k: f32) -> f32 {
    smooth_max(a, -b, k)
}

/// Chamfer minimum — creates a **flat chamfer** between two SDF surfaces.
///
/// `r` is the chamfer distance in model units.
/// Produces a 45-degree flat bevel rather than a round fillet.
#[inline]
pub fn chamfer_min(a: f32, b: f32, r: f32) -> f32 {
    union(union(a, b), (a - r + b) * std::f32::consts::FRAC_1_SQRT_2)
}

// ── SDF node wrappers ─────────────────────────────────────────────────────

/// SDF node: union of two children (sharp join, no blending).
pub struct UnionNode {
    pub a: SdfNode,
    pub b: SdfNode,
}
impl Sdf for UnionNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        union(self.a.distance(p), self.b.distance(p))
    }
}

/// SDF node: A minus B (sharp cut).
pub struct DifferenceNode {
    pub a: SdfNode,
    pub b: SdfNode,
}
impl Sdf for DifferenceNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        difference(self.a.distance(p), self.b.distance(p))
    }
}

/// SDF node: intersection of A and B.
pub struct IntersectionNode {
    pub a: SdfNode,
    pub b: SdfNode,
}
impl Sdf for IntersectionNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        intersection(self.a.distance(p), self.b.distance(p))
    }
}

/// SDF node: smooth union with fillet radius `k`.
pub struct SmoothUnionNode {
    pub a: SdfNode,
    pub b: SdfNode,
    pub k: f32,
}
impl Sdf for SmoothUnionNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        smooth_min(self.a.distance(p), self.b.distance(p), self.k)
    }
}

/// SDF node: smooth difference (rounded cutout).
pub struct SmoothDifferenceNode {
    pub a: SdfNode,
    pub b: SdfNode,
    pub k: f32,
}
impl Sdf for SmoothDifferenceNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        smooth_difference(self.a.distance(p), self.b.distance(p), self.k)
    }
}

/// SDF node: chamfer union (flat bevel join).
pub struct ChamferUnionNode {
    pub a: SdfNode,
    pub b: SdfNode,
    pub r: f32,
}
impl Sdf for ChamferUnionNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        chamfer_min(self.a.distance(p), self.b.distance(p), self.r)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn union_picks_minimum() {
        assert_abs_diff_eq!(union(3.0, 5.0), 3.0, epsilon = 1e-6);
        assert_abs_diff_eq!(union(-1.0, 2.0), -1.0, epsilon = 1e-6);
    }

    #[test]
    fn difference_max_neg_b() {
        // Point outside A, inside B → should be cut out (positive result)
        let a = 1.0_f32; // outside A
        let b = -1.0_f32; // inside B
        assert!(difference(a, b) > 0.0);
    }

    #[test]
    fn intersection_picks_maximum() {
        assert_abs_diff_eq!(intersection(3.0, 5.0), 5.0, epsilon = 1e-6);
    }

    #[test]
    fn smooth_min_degenerates_to_union_at_k_zero() {
        let a = 2.0_f32;
        let b = 4.0_f32;
        assert_abs_diff_eq!(smooth_min(a, b, 0.0), union(a, b), epsilon = 1e-5);
    }

    #[test]
    fn smooth_min_blends_between_values() {
        let a = 0.0_f32;
        let b = 0.0_f32;
        // When both are 0 (on the surface), smooth_min with k should give -(k/4)
        let result = smooth_min(a, b, 4.0);
        assert!(result < 0.0, "smooth_min at equal surfaces should dip below zero");
    }

    #[test]
    fn chamfer_min_is_no_larger_than_union() {
        let a = 3.0_f32;
        let b = 5.0_f32;
        assert!(chamfer_min(a, b, 1.0) <= union(a, b) + 1e-5);
    }
}
