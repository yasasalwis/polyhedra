//! PyO3 Python bindings for the polyhedra geometry kernel.
//!
//! Compiled only when the `python` feature is enabled (i.e. by maturin).
//! Exports a `_core` Python extension module with:
//!
//! - `SdfNode` class — type-erased SDF tree node, all geometry + operations
//! - `Mesh`    class — meshed geometry, ready for export
//! - Primitive constructors: `cube`, `sphere`, `cylinder`, `cone`,
//!                           `torus`, `pyramid`, `prism`
//! - `mesh_sdf(node, resolution, bounds)` — mesh an SDF into a `Mesh`
//! - `to_stl/obj/ply/glb(mesh)` — export to bytes
//! - `save_mesh(mesh, path)` — write to file (format inferred from extension)

use std::sync::Arc;

use glam::Vec3;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::error::PolyhedraError;
use crate::export::{ExportFormat, to_bytes, to_file};
use crate::mesher::{mesh as do_mesh, MeshConfig, Mesh};
use crate::sdf::Sdf;
use crate::sdf::primitives::{CubeSdf, SphereSdf, CylinderSdf, ConeSdf, TorusSdf, PyramidSdf, PrismSdf};
use crate::sdf::transform::{Mirror, MirrorPlane, Rotate, Scale, ScaleNonUniform, Translate};
use crate::sdf::operations::{
    UnionNode, DifferenceNode, IntersectionNode,
    SmoothUnionNode, SmoothDifferenceNode, ChamferUnionNode,
};
use crate::manipulations::{OffsetNode, ShellNode, ElongateNode};

// ── Arc<dyn Sdf> ↔ Box<dyn Sdf> bridge ───────────────────────────────────────

/// Thin wrapper that lets us store an `Arc<dyn Sdf>` inside a `Box<dyn Sdf>`.
///
/// All the compound node types (UnionNode, DifferenceNode, …) take ownership
/// of `Box<dyn Sdf>` children.  By wrapping our `Arc` in this struct we can
/// clone it cheaply and hand ownership to those nodes.
struct ArcSdf(Arc<dyn Sdf + Send + Sync>);

impl Sdf for ArcSdf {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        self.0.distance(p)
    }
}

/// Clone an `Arc<dyn Sdf>` into a `Box<dyn Sdf>`.
#[inline]
fn arc_to_box(a: &Arc<dyn Sdf + Send + Sync>) -> Box<dyn Sdf> {
    Box::new(ArcSdf(a.clone()))
}

// ── Error conversion ──────────────────────────────────────────────────────────

fn poly_err(e: PolyhedraError) -> PyErr {
    PyValueError::new_err(e.to_string())
}

// ── PySdfNode ─────────────────────────────────────────────────────────────────

/// A geometry node in the SDF tree.
///
/// Primitives, boolean operations, and manipulations all produce `SdfNode`
/// instances.  Nodes are reference-counted and cheaply cloned.
///
/// Example usage from Python::
///
///     import _core as ph
///     box_  = ph.cube(60, 40, 20)
///     shell = box_.shell(2.0)
///     mesh  = ph.mesh_sdf(shell, 24, 40.0)
///     data  = ph.to_stl(mesh)
#[pyclass(name = "SdfNode")]
pub struct PySdfNode {
    pub inner: Arc<dyn Sdf + Send + Sync>,
}

impl PySdfNode {
    fn new(sdf: impl Sdf + Send + Sync + 'static) -> Self {
        Self { inner: Arc::new(sdf) }
    }

    fn from_arc(arc: Arc<dyn Sdf + Send + Sync>) -> Self {
        Self { inner: arc }
    }

    fn boxes(&self, other: &PySdfNode) -> (Box<dyn Sdf>, Box<dyn Sdf>) {
        (arc_to_box(&self.inner), arc_to_box(&other.inner))
    }

    fn box_self(&self) -> Box<dyn Sdf> {
        arc_to_box(&self.inner)
    }
}

#[pymethods]
impl PySdfNode {

    // ── Boolean operations ────────────────────────────────────────────────

    /// Union: the combined volume of `self` and `other`.
    fn union(&self, other: &PySdfNode) -> PySdfNode {
        let (a, b) = self.boxes(other);
        PySdfNode::new(UnionNode { a, b })
    }

    /// Difference: `self` minus `other` (CSG subtraction).
    fn difference(&self, other: &PySdfNode) -> PySdfNode {
        let (a, b) = self.boxes(other);
        PySdfNode::new(DifferenceNode { a, b })
    }

    /// Intersection: only the volume shared by both `self` and `other`.
    fn intersection(&self, other: &PySdfNode) -> PySdfNode {
        let (a, b) = self.boxes(other);
        PySdfNode::new(IntersectionNode { a, b })
    }

    /// Smooth union with a fillet radius `k` (mm).  Higher `k` = softer blend.
    fn smooth_union(&self, other: &PySdfNode, k: f32) -> PySdfNode {
        let (a, b) = self.boxes(other);
        PySdfNode::new(SmoothUnionNode { a, b, k })
    }

    /// Smooth difference: rounded cutout of `other` from `self`.
    fn smooth_difference(&self, other: &PySdfNode, k: f32) -> PySdfNode {
        let (a, b) = self.boxes(other);
        PySdfNode::new(SmoothDifferenceNode { a, b, k })
    }

    /// Chamfer union: flat 45° bevel where the two surfaces meet.
    fn chamfer_union(&self, other: &PySdfNode, r: f32) -> PySdfNode {
        let (a, b) = self.boxes(other);
        PySdfNode::new(ChamferUnionNode { a, b, r })
    }

    // ── Morphological manipulations ───────────────────────────────────────

    /// Outward offset (positive) or inward shrink (negative).
    fn offset(&self, amount: f32) -> PySdfNode {
        PySdfNode::new(OffsetNode { inner: self.box_self(), offset: amount })
    }

    /// Hollow shell with given wall thickness.
    fn shell(&self, thickness: f32) -> PySdfNode {
        PySdfNode::new(ShellNode { inner: self.box_self(), thickness })
    }

    /// Elongate along each axis by the given amounts `(ex, ey, ez)`.
    fn elongate(&self, ex: f32, ey: f32, ez: f32) -> PySdfNode {
        PySdfNode::new(ElongateNode {
            inner:   self.box_self(),
            amounts: Vec3::new(ex, ey, ez),
        })
    }

    // ── Transforms ────────────────────────────────────────────────────────

    /// Translate by `(tx, ty, tz)`.
    fn translate(&self, tx: f32, ty: f32, tz: f32) -> PySdfNode {
        PySdfNode::new(Translate {
            inner:  self.box_self(),
            offset: Vec3::new(tx, ty, tz),
        })
    }

    /// Uniform scale by `factor`.
    fn scale(&self, factor: f32) -> PySdfNode {
        PySdfNode::new(Scale { inner: self.box_self(), factor })
    }

    /// Non-uniform scale by `(sx, sy, sz)`.
    fn scale_xyz(&self, sx: f32, sy: f32, sz: f32) -> PySdfNode {
        PySdfNode::new(ScaleNonUniform {
            inner: self.box_self(),
            scale: Vec3::new(sx, sy, sz),
        })
    }

    /// Rotate about X axis by `degrees`.
    fn rotate_x(&self, degrees: f32) -> PySdfNode {
        let q = glam::Quat::from_rotation_x(degrees.to_radians());
        PySdfNode::new(Rotate::new(self.box_self(), q))
    }

    /// Rotate about Y axis by `degrees`.
    fn rotate_y(&self, degrees: f32) -> PySdfNode {
        let q = glam::Quat::from_rotation_y(degrees.to_radians());
        PySdfNode::new(Rotate::new(self.box_self(), q))
    }

    /// Rotate about Z axis by `degrees`.
    fn rotate_z(&self, degrees: f32) -> PySdfNode {
        let q = glam::Quat::from_rotation_z(degrees.to_radians());
        PySdfNode::new(Rotate::new(self.box_self(), q))
    }

    /// Mirror across the YZ plane (flip X).
    fn mirror_x(&self) -> PySdfNode {
        PySdfNode::new(Mirror { inner: self.box_self(), plane: MirrorPlane::Yz })
    }

    /// Mirror across the XZ plane (flip Y).
    fn mirror_y(&self) -> PySdfNode {
        PySdfNode::new(Mirror { inner: self.box_self(), plane: MirrorPlane::Xz })
    }

    /// Mirror across the XY plane (flip Z).
    fn mirror_z(&self) -> PySdfNode {
        PySdfNode::new(Mirror { inner: self.box_self(), plane: MirrorPlane::Xy })
    }

    // ── Evaluation ────────────────────────────────────────────────────────

    /// Evaluate the SDF at world-space point `(x, y, z)`.
    ///
    /// Returns a negative value inside, zero on the surface, positive outside.
    fn distance(&self, x: f32, y: f32, z: f32) -> f32 {
        self.inner.distance(Vec3::new(x, y, z))
    }

    fn __repr__(&self) -> String {
        "SdfNode(<geometry>)".to_string()
    }
}

// ── PyMesh ────────────────────────────────────────────────────────────────────

/// A triangulated mesh produced by the Dual Contouring mesher.
///
/// Call `mesh_sdf()` to create one.  Use `to_stl()`, `to_obj()`, `to_ply()`,
/// `to_glb()`, or `save()` to export.
#[pyclass(name = "Mesh")]
pub struct PyMesh {
    pub inner: Mesh,
}

#[pymethods]
impl PyMesh {
    /// Number of vertices in the mesh.
    fn vertex_count(&self) -> usize {
        self.inner.vertex_count()
    }

    /// Number of triangles in the mesh.
    fn triangle_count(&self) -> usize {
        self.inner.triangle_count()
    }

    /// Export to binary STL bytes.
    fn to_stl(&self) -> PyResult<Vec<u8>> {
        to_bytes(&self.inner, ExportFormat::Stl).map_err(poly_err)
    }

    /// Export to OBJ text (returned as `bytes`).
    fn to_obj(&self) -> PyResult<Vec<u8>> {
        to_bytes(&self.inner, ExportFormat::Obj).map_err(poly_err)
    }

    /// Export to binary PLY bytes.
    fn to_ply(&self) -> PyResult<Vec<u8>> {
        to_bytes(&self.inner, ExportFormat::Ply).map_err(poly_err)
    }

    /// Export to binary GLB (glTF 2.0) bytes.
    fn to_glb(&self) -> PyResult<Vec<u8>> {
        to_bytes(&self.inner, ExportFormat::Glb).map_err(poly_err)
    }

    /// Write the mesh to `path`, inferring the format from the file extension.
    ///
    /// Supported extensions: `.stl`, `.obj`, `.ply`, `.glb`, `.gltf`.
    fn save(&self, path: &str) -> PyResult<()> {
        to_file(&self.inner, path).map_err(poly_err)
    }

    fn __repr__(&self) -> String {
        format!(
            "Mesh(vertices={}, triangles={})",
            self.inner.vertex_count(),
            self.inner.triangle_count()
        )
    }
}

// ── Primitive constructors ────────────────────────────────────────────────────

/// Axis-aligned box centred at the origin.
///
/// Args:
///     width  (float): full dimension along X (mm).
///     depth  (float): full dimension along Y (mm).
///     height (float): full dimension along Z (mm).
#[pyfunction]
fn cube(width: f32, depth: f32, height: f32) -> PySdfNode {
    PySdfNode::new(CubeSdf::new(width, depth, height))
}

/// Sphere centred at the origin.
///
/// Args:
///     radius (float): sphere radius (mm).
#[pyfunction]
fn sphere(radius: f32) -> PySdfNode {
    PySdfNode::new(SphereSdf::new(radius))
}

/// Capped cylinder centred at the origin, axis along Z.
///
/// Args:
///     radius (float): cylinder radius (mm).
///     height (float): full height (mm).
#[pyfunction]
fn cylinder(radius: f32, height: f32) -> PySdfNode {
    PySdfNode::new(CylinderSdf::new(radius, height))
}

/// Cone (or truncated cone) centred at the origin, axis along Z.
///
/// Args:
///     base_radius (float): radius at the bottom (-Z) face (mm).
///     top_radius  (float): radius at the top (+Z) face (mm).  Use 0 for a sharp apex.
///     height      (float): full height (mm).
#[pyfunction]
fn cone(base_radius: f32, top_radius: f32, height: f32) -> PySdfNode {
    PySdfNode::new(ConeSdf::new(base_radius, top_radius, height))
}

/// Torus centred at the origin, ring in the XY plane.
///
/// Args:
///     major (float): distance from origin to tube centre (mm).
///     minor (float): tube cross-section radius (mm).
#[pyfunction]
fn torus(major: f32, minor: f32) -> PySdfNode {
    PySdfNode::new(TorusSdf::new(major, minor))
}

/// Rectangular-base pyramid centred at the origin, apex at +Z.
///
/// Args:
///     base_width (float): full width (X) of the rectangular base (mm).
///     base_depth (float): full depth (Y) of the rectangular base (mm).
///     height     (float): full height (mm).
#[pyfunction]
fn pyramid(base_width: f32, base_depth: f32, height: f32) -> PySdfNode {
    PySdfNode::new(PyramidSdf::new(base_width, base_depth, height))
}

/// Regular n-sided prism centred at the origin, axis along Z.
///
/// Args:
///     sides        (int):   number of polygon sides (≥ 3).
///     flat_to_flat (float): flat-to-flat diameter (mm) — the inscribed circle diameter.
///     height       (float): full height (mm).
#[pyfunction]
fn prism(sides: u32, flat_to_flat: f32, height: f32) -> PySdfNode {
    PySdfNode::new(PrismSdf::new(sides, flat_to_flat, height))
}

// ── Meshing ───────────────────────────────────────────────────────────────────

/// Mesh an SDF node using Dual Contouring.
///
/// Args:
///     node       (SdfNode): the geometry to mesh.
///     resolution (int):     number of voxels along each axis (e.g. 32).
///     bounds     (float):   half-size of the meshing volume in mm.
///                           The meshing box is `[-bounds, +bounds]³`.
///
/// Returns:
///     Mesh: the triangulated mesh.
///
/// Raises:
///     ValueError: if the resulting mesh is empty.
#[pyfunction]
fn mesh_sdf(node: &PySdfNode, resolution: u32, bounds: f32) -> PyResult<PyMesh> {
    let cfg = MeshConfig::centered(bounds, resolution);
    let inner = do_mesh(node.inner.as_ref(), &cfg);
    if inner.triangle_count() == 0 {
        return Err(PyValueError::new_err(
            "mesh_sdf produced an empty mesh — \
             try increasing `bounds` or `resolution`, \
             or check that your SDF contains geometry."
        ));
    }
    Ok(PyMesh { inner })
}

// ── Module registration ───────────────────────────────────────────────────────

/// Register all Python-facing symbols into the `_core` PyO3 module.
///
/// Called from the `#[pymodule]` entry point in `lib.rs`.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<PySdfNode>()?;
    m.add_class::<PyMesh>()?;

    // Primitive constructors
    m.add_function(wrap_pyfunction!(cube,     m)?)?;
    m.add_function(wrap_pyfunction!(sphere,   m)?)?;
    m.add_function(wrap_pyfunction!(cylinder, m)?)?;
    m.add_function(wrap_pyfunction!(cone,     m)?)?;
    m.add_function(wrap_pyfunction!(torus,    m)?)?;
    m.add_function(wrap_pyfunction!(pyramid,  m)?)?;
    m.add_function(wrap_pyfunction!(prism,    m)?)?;

    // Meshing
    m.add_function(wrap_pyfunction!(mesh_sdf, m)?)?;

    Ok(())
}
