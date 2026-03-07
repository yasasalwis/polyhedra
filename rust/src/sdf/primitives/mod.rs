//! SDF primitive shapes.
//!
//! Each primitive is a Rust struct that implements the [`crate::sdf::Sdf`] trait.
//! Primitives are centred at the origin; use a [`crate::sdf::TransformedSdf`]
//! to position them in world space.
//!
//! Phases add one primitive per phase:
//!   Phase 1  — [`CubeSdf`]       (axis-aligned box)
//!   Phase 2  — CylinderSdf
//!   Phase 3  — SphereSdf
//!   Phase 4  — ConeSdf
//!   Phase 5  — TorusSdf
//!   Phase 6  — PyramidSdf
//!   Phase 7  — PrismSdf

pub mod cone;
pub mod cube;
pub mod cylinder;
pub mod prism;
pub mod pyramid;
pub mod sphere;
pub mod torus;

pub use cone::ConeSdf;
pub use cube::CubeSdf;
pub use cylinder::CylinderSdf;
pub use prism::PrismSdf;
pub use pyramid::PyramidSdf;
pub use sphere::SphereSdf;
pub use torus::TorusSdf;
