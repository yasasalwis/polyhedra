//! SDF primitive shapes.
//!
//! Each primitive is a Rust struct that implements the [`crate::sdf::Sdf`] trait.
//! Primitives are centred at the origin; use a [`crate::sdf::TransformedSdf`]
//! to position them in world space.

pub mod cone;
pub mod cube;
pub mod cylinder;
pub mod gear;
pub mod prism;
pub mod pyramid;
pub mod sphere;
pub mod torus;

pub use cone::ConeSdf;
pub use cube::CubeSdf;
pub use cylinder::CylinderSdf;
pub use gear::GearSdf;
pub use prism::PrismSdf;
pub use pyramid::PyramidSdf;
pub use sphere::SphereSdf;
pub use torus::TorusSdf;
