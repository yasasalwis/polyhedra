//! Dual Contouring mesher — converts SDF trees into triangle meshes.
//!
//! # Public API
//!
//! ```
//! use _core::mesher::{mesh, MeshConfig};
//! use _core::sdf::primitives::SphereSdf;
//!
//! let sdf = SphereSdf::new(5.0);
//! let cfg = MeshConfig::centered(7.0, 32);
//! let m   = mesh(&sdf, &cfg);
//! assert!(m.vertex_count()   > 0);
//! assert!(m.triangle_count() > 0);
//! assert!(m.is_index_valid());
//! ```

pub mod dc;
pub mod qef;

pub use dc::{Mesh, MeshConfig, mesh};
