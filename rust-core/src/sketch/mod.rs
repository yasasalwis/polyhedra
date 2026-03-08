//! 2D Sketch system — closed profiles and 3D extrusion operations.
//!
//! A *sketch* is a 2D signed distance function.  The [`Sdf2d`] trait mirrors
//! the 3D [`crate::sdf::Sdf`] trait but operates on [`glam::Vec2`] points.
//!
//! # Primitives
//!
//! | Type              | Shape                                    |
//! |-------------------|------------------------------------------|
//! [`Circle`]          | Circle of given radius                   |
//! [`Rect`]            | Axis-aligned rectangle                   |
//! [`RoundedRect`]     | Rectangle with uniform corner radius     |
//! [`RegularPolygon`]  | Regular n-gon (same metric as PrismSdf)  |
//! [`Capsule`]         | Swept circle along a line segment        |
//!
//! # 3D Operations
//!
//! | Type            | Operation                                                |
//! |-----------------|----------------------------------------------------------|
//! [`ExtrudeNode`]   | Extrude profile along Z — yields exact Euclidean SDF    |
//! [`RevolveNode`]   | Revolve profile around an axis — yields exact SDF        |
//!
//! # Relationship to built-in primitives
//!
//! The 3D operations are mathematically equivalent to the primitive SDFs:
//!
//! - `ExtrudeNode { Circle(r), height }` = `CylinderSdf { radius: r, height }`
//! - `RevolveNode { Circle(minor), Z, offset: major }` = `TorusSdf { major, minor }`
//! - `ExtrudeNode { Rect(w, d), height }` = `CubeSdf { w, d, height }`
//! - `ExtrudeNode { RegularPolygon(n, a), height }` = `PrismSdf { n, flat_to_flat: 2a, height }`
//!
//! The sketch system lets users define *arbitrary* closed profiles beyond the
//! built-in primitives and pipe them through the same extrusion/revolve math.

pub mod ops;
pub mod primitives;

use glam::Vec2;

pub use ops::{ExtrudeNode, RevolveAxis, RevolveNode};
pub use primitives::{Capsule, Circle, Rect, RegularPolygon, RoundedRect};

// ── 2D SDF trait ──────────────────────────────────────────────────────────────

/// Signed distance function in the XY plane.
///
/// Convention identical to the 3D [`crate::sdf::Sdf`]:
/// - Negative inside, zero on boundary, positive outside.
pub trait Sdf2d: Send + Sync {
    fn distance(&self, p: Vec2) -> f32;
}

/// Heap-allocated, type-erased 2D SDF node.
pub type Sdf2dNode = Box<dyn Sdf2d>;

// ── 2D Boolean operations ─────────────────────────────────────────────────────

/// Union of two 2D SDFs.
pub struct Union2d {
    pub a: Sdf2dNode,
    pub b: Sdf2dNode,
}
impl Sdf2d for Union2d {
    #[inline]
    fn distance(&self, p: Vec2) -> f32 {
        self.a.distance(p).min(self.b.distance(p))
    }
}

/// Difference: a minus b.
pub struct Difference2d {
    pub a: Sdf2dNode,
    pub b: Sdf2dNode,
}
impl Sdf2d for Difference2d {
    #[inline]
    fn distance(&self, p: Vec2) -> f32 {
        self.a.distance(p).max(-self.b.distance(p))
    }
}

/// Intersection of two 2D SDFs.
pub struct Intersection2d {
    pub a: Sdf2dNode,
    pub b: Sdf2dNode,
}
impl Sdf2d for Intersection2d {
    #[inline]
    fn distance(&self, p: Vec2) -> f32 {
        self.a.distance(p).max(self.b.distance(p))
    }
}

/// Offset (expand / contract) a 2D SDF.
pub struct Offset2d {
    pub inner: Sdf2dNode,
    pub offset: f32,
}
impl Sdf2d for Offset2d {
    #[inline]
    fn distance(&self, p: Vec2) -> f32 {
        self.inner.distance(p) - self.offset
    }
}
