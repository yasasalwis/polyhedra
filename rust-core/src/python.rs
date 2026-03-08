//! PyO3 Python bindings for the polyhedra geometry kernel.
//!
//! Compiled only when the `python` feature is enabled (i.e. by maturin).
//! Exports a `_core` Python extension module with:
//!
//! - `SdfNode` class — type-erased SDF tree node, all geometry + operations
//! - `Mesh`    class — meshed geometry, ready for export
//! - Primitive constructors: `cube`, `sphere`, `cylinder`, `cone`,
//!                           `torus`, `pyramid`, `prism`, `gear`
//! - Engineering parts: `thread`, `spring`, `knurl`, `spline`, `i_beam`,
//!                      `t_slot`, `rack`, `sprocket`, `bearing`, `cam`,
//!                      `dovetail`, `csk_hole`, `hex_bolt`, `star`, `cross_section`
//! - `mesh_sdf(node, resolution, bounds)` — mesh an SDF into a `Mesh`
//! - `to_stl/obj/ply/glb(mesh)` — export to bytes
//! - `save_mesh(mesh, path)` — write to file (format inferred from extension)

use std::sync::Arc;

use glam::Vec3;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::error::PolyhedraError;
use crate::export::{ExportFormat, to_bytes, to_file};
use crate::manipulations::{ElongateNode, OffsetNode, ShellNode};
use crate::mesher::{Mesh, MeshConfig, mesh as do_mesh};
use crate::sdf::Sdf;
use crate::sdf::operations::{
    ChamferUnionNode, DifferenceNode, IntersectionNode, SmoothDifferenceNode, SmoothUnionNode,
    UnionNode,
};
use crate::sdf::parts::{
    BearingSdf, CamSdf, CrossSectionSdf, CskHoleSdf, DovetailSdf, HexBoltSdf, IBeamSdf,
    KnurlSdf, RackSdf, SplineSdf, SpringSdf, SprocketSdf, StarSdf, ThreadSdf, TSlotSdf,
};
use crate::sdf::primitives::{
    ConeSdf, CubeSdf, CylinderSdf, GearSdf, PrismSdf, PyramidSdf, SphereSdf, TorusSdf,
};
use crate::sdf::transform::{Mirror, MirrorPlane, Rotate, Scale, ScaleNonUniform, Translate};

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
        Self {
            inner: Arc::new(sdf),
        }
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
        PySdfNode::new(OffsetNode {
            inner: self.box_self(),
            offset: amount,
        })
    }

    /// Hollow shell with given wall thickness.
    fn shell(&self, thickness: f32) -> PySdfNode {
        PySdfNode::new(ShellNode {
            inner: self.box_self(),
            thickness,
        })
    }

    /// Elongate along each axis by the given amounts `(ex, ey, ez)`.
    fn elongate(&self, ex: f32, ey: f32, ez: f32) -> PySdfNode {
        PySdfNode::new(ElongateNode {
            inner: self.box_self(),
            amounts: Vec3::new(ex, ey, ez),
        })
    }

    // ── Transforms ────────────────────────────────────────────────────────

    /// Translate by `(tx, ty, tz)`.
    fn translate(&self, tx: f32, ty: f32, tz: f32) -> PySdfNode {
        PySdfNode::new(Translate {
            inner: self.box_self(),
            offset: Vec3::new(tx, ty, tz),
        })
    }

    /// Uniform scale by `factor`.
    fn scale(&self, factor: f32) -> PySdfNode {
        PySdfNode::new(Scale {
            inner: self.box_self(),
            factor,
        })
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
        PySdfNode::new(Mirror {
            inner: self.box_self(),
            plane: MirrorPlane::Yz,
        })
    }

    /// Mirror across the XZ plane (flip Y).
    fn mirror_y(&self) -> PySdfNode {
        PySdfNode::new(Mirror {
            inner: self.box_self(),
            plane: MirrorPlane::Xz,
        })
    }

    /// Mirror across the XY plane (flip Z).
    fn mirror_z(&self) -> PySdfNode {
        PySdfNode::new(Mirror {
            inner: self.box_self(),
            plane: MirrorPlane::Xy,
        })
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

/// Spur gear centred at the origin, axis along Z.
///
/// Args:
///     teeth         (int):   number of teeth (≥ 3).
///     pitch_radius  (float): pitch circle radius (mm).
///     tooth_height  (float): radial height of each tooth (mm).
///     tooth_fraction (float): tooth width as fraction of pitch (0..1, default 0.5).
///     height        (float): full gear thickness (mm).
#[pyfunction]
fn gear(teeth: u32, pitch_radius: f32, tooth_height: f32, tooth_fraction: f32, height: f32) -> PySdfNode {
    PySdfNode::new(GearSdf::new(teeth, pitch_radius, tooth_height, tooth_fraction, height))
}

/// Metric threaded rod centred at the origin, axis along Z.
///
/// Args:
///     outer_radius (float): major (outer) radius (mm).
///     pitch        (float): thread pitch — mm per turn.
///     height       (float): total rod length (mm).
#[pyfunction]
fn thread(outer_radius: f32, pitch: f32, height: f32) -> PySdfNode {
    PySdfNode::new(ThreadSdf::new(outer_radius, pitch, height))
}

/// Coil spring centred at the origin, axis along Z.
///
/// Args:
///     coil_radius  (float): radius from spring axis to wire centre (mm).
///     wire_radius  (float): wire cross-section radius (mm).
///     pitch        (float): axial distance per turn (mm).
///     turns        (float): number of coils.
#[pyfunction]
fn spring(coil_radius: f32, wire_radius: f32, pitch: f32, turns: f32) -> PySdfNode {
    PySdfNode::new(SpringSdf::new(coil_radius, wire_radius, pitch, turns))
}

/// Knurled cylinder centred at the origin, axis along Z.
///
/// Args:
///     radius      (float): base cylinder radius (mm).
///     height      (float): total height (mm).
///     bump_depth  (float): bump amplitude (mm).
///     n_rows      (int):   number of rows around circumference.
///     pitch       (float): axial bump pitch (mm).
#[pyfunction]
fn knurl(radius: f32, height: f32, bump_depth: f32, n_rows: u32, pitch: f32) -> PySdfNode {
    PySdfNode::new(KnurlSdf::new(radius, height, bump_depth, n_rows, pitch))
}

/// Splined shaft centred at the origin, axis along Z.
///
/// Args:
///     pitch_radius   (float): base circle radius (mm).
///     tooth_height   (float): radial height of each spline tooth (mm).
///     height         (float): shaft length (mm).
///     n_splines      (int):   number of splines.
///     tooth_fraction (float): tooth width fraction (0..1).
#[pyfunction]
fn spline(pitch_radius: f32, tooth_height: f32, height: f32, n_splines: u32, tooth_fraction: f32) -> PySdfNode {
    PySdfNode::new(SplineSdf::new(pitch_radius, tooth_height, height, n_splines, tooth_fraction))
}

/// I-beam / H-beam centred at the origin, extruded along Z.
///
/// Args:
///     flange_width     (float): full flange width (mm).
///     flange_thickness (float): flange plate thickness (mm).
///     web_height       (float): web height between flanges (mm).
///     web_thickness    (float): web plate thickness (mm).
///     length           (float): beam length along Z (mm).
#[pyfunction]
fn i_beam(flange_width: f32, flange_thickness: f32, web_height: f32, web_thickness: f32, length: f32) -> PySdfNode {
    PySdfNode::new(IBeamSdf::new(flange_width, flange_thickness, web_height, web_thickness, length))
}

/// T-slot aluminium extrusion centred at the origin, extruded along Z.
///
/// Args:
///     side            (float): square cross-section side length (mm).
///     slot_width      (float): T-slot mouth width (mm).
///     slot_head_width (float): T-slot head (inner) width (mm).
///     slot_depth      (float): T-slot depth from face (mm).
///     length          (float): extrusion length along Z (mm).
#[pyfunction]
fn t_slot(side: f32, slot_width: f32, slot_head_width: f32, slot_depth: f32, length: f32) -> PySdfNode {
    PySdfNode::new(TSlotSdf::new(side, slot_width, slot_head_width, slot_depth, length))
}

/// Gear rack centred at the origin, teeth on the +Y face.
///
/// Args:
///     length         (float): rack length along X (mm).
///     width          (float): bar width along Y (mm).
///     height         (float): bar height along Z (mm).
///     tooth_height   (float): tooth height above bar (mm).
///     pitch          (float): tooth pitch along X (mm).
///     tooth_fraction (float): tooth width / pitch (0..1).
#[pyfunction]
fn rack(length: f32, width: f32, height: f32, tooth_height: f32, pitch: f32, tooth_fraction: f32) -> PySdfNode {
    PySdfNode::new(RackSdf::new(length, width, height, tooth_height, pitch, tooth_fraction))
}

/// Chain sprocket centred at the origin, axis along Z.
///
/// Args:
///     pitch_radius (float): radius to tooth tips base circle (mm).
///     tooth_height (float): radial tooth height (mm).
///     bore_radius  (float): central bore radius (mm).
///     height       (float): sprocket thickness (mm).
///     n_teeth      (int):   number of teeth.
#[pyfunction]
fn sprocket(pitch_radius: f32, tooth_height: f32, bore_radius: f32, height: f32, n_teeth: u32) -> PySdfNode {
    PySdfNode::new(SprocketSdf::new(pitch_radius, tooth_height, bore_radius, height, n_teeth))
}

/// Ball bearing centred at the origin, axis along Z.
///
/// Args:
///     outer_radius (float): outer race outer radius (mm).
///     inner_radius (float): inner race inner radius (mm).
///     height       (float): bearing height (mm).
///     n_balls      (int):   number of rolling balls.
#[pyfunction]
fn bearing(outer_radius: f32, inner_radius: f32, height: f32, n_balls: u32) -> PySdfNode {
    PySdfNode::new(BearingSdf::new(outer_radius, inner_radius, height, n_balls))
}

/// Eccentric disc cam centred at the origin, extruded along Z.
///
/// Args:
///     cam_radius    (float): disc radius (mm).
///     eccentricity  (float): offset of disc centre from origin (mm).
///     height        (float): cam thickness (mm).
#[pyfunction]
fn cam(cam_radius: f32, eccentricity: f32, height: f32) -> PySdfNode {
    PySdfNode::new(CamSdf::new(cam_radius, eccentricity, height))
}

/// Dovetail slide centred at the origin, extruded along Z.
///
/// Args:
///     top_width      (float): narrow end width (mm).
///     bottom_width   (float): wide end width (mm).
///     profile_height (float): trapezoidal cross-section height (mm).
///     length         (float): extrusion length along Z (mm).
#[pyfunction]
fn dovetail(top_width: f32, bottom_width: f32, profile_height: f32, length: f32) -> PySdfNode {
    PySdfNode::new(DovetailSdf::new(top_width, bottom_width, profile_height, length))
}

/// Countersunk hole opening at z=0, boring down the -Z axis.
///
/// The SDF is negative (interior) inside the hole — combine with `difference`
/// to cut this from a solid body.
///
/// Args:
///     bore_diameter (float): cylindrical bore diameter (mm).
///     csk_diameter  (float): countersink outer diameter at the surface (mm).
///     csk_depth     (float): axial depth of the conical countersink (mm).
///     total_depth   (float): total hole depth (mm).
#[pyfunction]
fn csk_hole(bore_diameter: f32, csk_diameter: f32, csk_depth: f32, total_depth: f32) -> PySdfNode {
    PySdfNode::new(CskHoleSdf::new(bore_diameter, csk_diameter, csk_depth, total_depth))
}

/// Hex bolt with origin at the shank tip, axis along +Z through shank then head.
///
/// Args:
///     across_flats  (float): hex head AF dimension (mm).
///     head_height   (float): head height (mm).
///     shank_diameter (float): shank diameter (mm).
///     shank_length  (float): shank length (mm).
#[pyfunction]
fn hex_bolt(across_flats: f32, head_height: f32, shank_diameter: f32, shank_length: f32) -> PySdfNode {
    PySdfNode::new(HexBoltSdf::new(across_flats, head_height, shank_diameter, shank_length))
}

/// N-pointed star prism centred at the origin, extruded along Z.
///
/// Args:
///     outer_radius (float): tip radius (mm).
///     inner_radius (float): valley radius (mm).
///     n_points     (int):   number of star points.
///     height       (float): extrusion height (mm).
#[pyfunction]
fn star(outer_radius: f32, inner_radius: f32, n_points: u32, height: f32) -> PySdfNode {
    PySdfNode::new(StarSdf::new(outer_radius, inner_radius, n_points, height))
}

/// Plus/cross profile prism centred at the origin, extruded along Z.
///
/// Args:
///     arm_width  (float): width of each arm (mm).
///     arm_length (float): full length of each arm from tip to tip (mm).
///     height     (float): extrusion height (mm).
#[pyfunction]
fn cross_section(arm_width: f32, arm_length: f32, height: f32) -> PySdfNode {
    PySdfNode::new(CrossSectionSdf::new(arm_width, arm_length, height))
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
             or check that your SDF contains geometry.",
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
    m.add_function(wrap_pyfunction!(cube, m)?)?;
    m.add_function(wrap_pyfunction!(sphere, m)?)?;
    m.add_function(wrap_pyfunction!(cylinder, m)?)?;
    m.add_function(wrap_pyfunction!(cone, m)?)?;
    m.add_function(wrap_pyfunction!(torus, m)?)?;
    m.add_function(wrap_pyfunction!(pyramid, m)?)?;
    m.add_function(wrap_pyfunction!(prism, m)?)?;
    m.add_function(wrap_pyfunction!(gear, m)?)?;

    // Engineering part constructors
    m.add_function(wrap_pyfunction!(thread, m)?)?;
    m.add_function(wrap_pyfunction!(spring, m)?)?;
    m.add_function(wrap_pyfunction!(knurl, m)?)?;
    m.add_function(wrap_pyfunction!(spline, m)?)?;
    m.add_function(wrap_pyfunction!(i_beam, m)?)?;
    m.add_function(wrap_pyfunction!(t_slot, m)?)?;
    m.add_function(wrap_pyfunction!(rack, m)?)?;
    m.add_function(wrap_pyfunction!(sprocket, m)?)?;
    m.add_function(wrap_pyfunction!(bearing, m)?)?;
    m.add_function(wrap_pyfunction!(cam, m)?)?;
    m.add_function(wrap_pyfunction!(dovetail, m)?)?;
    m.add_function(wrap_pyfunction!(csk_hole, m)?)?;
    m.add_function(wrap_pyfunction!(hex_bolt, m)?)?;
    m.add_function(wrap_pyfunction!(star, m)?)?;
    m.add_function(wrap_pyfunction!(cross_section, m)?)?;

    // Meshing
    m.add_function(wrap_pyfunction!(mesh_sdf, m)?)?;

    Ok(())
}
