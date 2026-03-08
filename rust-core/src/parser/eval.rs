//! AST → SDF evaluator for the `.polyh` DSL.
//!
//! Converts the parser's AST types into live `Box<dyn Sdf>` trees that the
//! mesher can consume.
//!
//! # Entry points
//! - [`eval_define`]   — evaluate a single `define` block.
//! - [`eval_assemble`] — evaluate an `assemble` block (needs a define lookup table).
//! - [`eval_file`]     — convenience wrapper: parse + evaluate the first `define`.

use std::collections::HashMap;

use glam::Vec3;

use crate::error::{PolyhedraError, Result};
use crate::manipulations::{OffsetNode, ShellNode};
use crate::sdf::SdfNode;
use crate::sdf::operations::{DifferenceNode, IntersectionNode, UnionNode};
use crate::sdf::parts::{
    BearingSdf, CamSdf, CrossSectionSdf, CskHoleSdf, DovetailSdf, HexBoltSdf, IBeamSdf, KnurlSdf,
    RackSdf, SplineSdf, SpringSdf, SprocketSdf, StarSdf, TSlotSdf, ThreadSdf,
};
use crate::sdf::primitives::{
    ConeSdf, CubeSdf, CylinderSdf, GearSdf, PrismSdf, PyramidSdf, SphereSdf, TorusSdf,
};
use crate::sdf::transform::Translate;

use super::ast::*;

// ── Unit scale ─────────────────────────────────────────────────────────────────

fn unit_scale(u: Unit) -> f32 {
    u.to_mm()
}

/// Return preset (key, value) pairs for the given primitive style.
/// The user's own `prop` values always take priority over these.
fn style_defaults(kind: PrimKind, style: &str) -> &'static [(&'static str, f32)] {
    match (kind, style) {
        // ── Cube ──────────────────────────────────────────────────────────────
        (PrimKind::Cube, "plate") => &[("width", 100.0), ("depth", 100.0), ("height", 5.0)],
        (PrimKind::Cube, "bar") => &[("width", 10.0), ("depth", 10.0), ("height", 100.0)],
        (PrimKind::Cube, "block") => &[("width", 40.0), ("depth", 40.0), ("height", 40.0)],
        (PrimKind::Cube, "beam") => &[("width", 20.0), ("depth", 30.0), ("height", 150.0)],
        (PrimKind::Cube, "shim") => &[("width", 30.0), ("depth", 30.0), ("height", 0.5)],
        (PrimKind::Cube, "key_stock") => &[("width", 8.0), ("depth", 8.0), ("height", 60.0)],
        (PrimKind::Cube, "fin") => &[("width", 2.0), ("depth", 40.0), ("height", 50.0)],
        (PrimKind::Cube, "gusset") => &[("width", 40.0), ("depth", 5.0), ("height", 40.0)],
        (PrimKind::Cube, "tile") => &[("width", 50.0), ("depth", 50.0), ("height", 8.0)],
        (PrimKind::Cube, "lug") => &[("width", 25.0), ("depth", 10.0), ("height", 35.0)],
        (PrimKind::Cube, "spacer_block") => &[("width", 20.0), ("depth", 20.0), ("height", 10.0)],
        (PrimKind::Cube, "bracket_tab") => &[("width", 40.0), ("depth", 5.0), ("height", 30.0)],
        (PrimKind::Cube, "rail") => &[("width", 15.0), ("depth", 12.0), ("height", 120.0)],
        (PrimKind::Cube, "chip") => &[("width", 8.0), ("depth", 5.0), ("height", 2.0)],
        (PrimKind::Cube, "wedge") => &[("width", 60.0), ("depth", 30.0), ("height", 20.0)],

        // ── Sphere ────────────────────────────────────────────────────────────
        (PrimKind::Sphere, "ball") => &[("radius", 10.0)],
        (PrimKind::Sphere, "marble") => &[("radius", 5.0)],
        (PrimKind::Sphere, "bearing_ball") => &[("radius", 3.0)],
        (PrimKind::Sphere, "knob") => &[("radius", 20.0)],
        (PrimKind::Sphere, "rivet_head") => &[("radius", 5.0)],
        (PrimKind::Sphere, "hemisphere") => &[("radius", 20.0)],
        (PrimKind::Sphere, "dome") => &[("radius", 50.0)],
        (PrimKind::Sphere, "lens") => &[("radius", 15.0)],
        (PrimKind::Sphere, "bubble") => &[("radius", 25.0)],
        (PrimKind::Sphere, "ball_joint") => &[("radius", 12.0)],
        (PrimKind::Sphere, "cam_follower") => &[("radius", 8.0)],
        (PrimKind::Sphere, "float_ball") => &[("radius", 30.0)],
        (PrimKind::Sphere, "shot") => &[("radius", 2.0)],
        (PrimKind::Sphere, "plug_end") => &[("radius", 8.0)],
        (PrimKind::Sphere, "sphere_cap") => &[("radius", 20.0)],

        // ── Cylinder ──────────────────────────────────────────────────────────
        (PrimKind::Cylinder, "shaft") => &[("radius", 5.0), ("height", 100.0)],
        (PrimKind::Cylinder, "rod") => &[("radius", 8.0), ("height", 60.0)],
        (PrimKind::Cylinder, "disk") => &[("radius", 30.0), ("height", 5.0)],
        (PrimKind::Cylinder, "puck") => &[("radius", 20.0), ("height", 15.0)],
        (PrimKind::Cylinder, "tube") => &[("radius", 15.0), ("height", 60.0)],
        (PrimKind::Cylinder, "bushing") => &[("radius", 12.0), ("height", 20.0)],
        (PrimKind::Cylinder, "pin") => &[("radius", 4.0), ("height", 25.0)],
        (PrimKind::Cylinder, "boss") => &[("radius", 8.0), ("height", 12.0)],
        (PrimKind::Cylinder, "collar") => &[("radius", 15.0), ("height", 10.0)],
        (PrimKind::Cylinder, "barrel") => &[("radius", 20.0), ("height", 35.0)],
        (PrimKind::Cylinder, "piston") => &[("radius", 25.0), ("height", 40.0)],
        (PrimKind::Cylinder, "reel") => &[("radius", 30.0), ("height", 50.0)],
        (PrimKind::Cylinder, "slug") => &[("radius", 12.0), ("height", 12.0)],
        (PrimKind::Cylinder, "plug") => &[("radius", 10.0), ("height", 20.0)],
        (PrimKind::Cylinder, "cap_end") => &[("radius", 15.0), ("height", 8.0)],

        // ── Cone ──────────────────────────────────────────────────────────────
        (PrimKind::Cone, "sharp") => {
            &[("base_radius", 15.0), ("top_radius", 0.0), ("height", 40.0)]
        }
        (PrimKind::Cone, "frustum") => &[
            ("base_radius", 20.0),
            ("top_radius", 12.0),
            ("height", 25.0),
        ],
        (PrimKind::Cone, "nozzle") => {
            &[("base_radius", 20.0), ("top_radius", 5.0), ("height", 30.0)]
        }
        (PrimKind::Cone, "funnel") => {
            &[("base_radius", 40.0), ("top_radius", 8.0), ("height", 50.0)]
        }
        (PrimKind::Cone, "countersink") => {
            &[("base_radius", 12.0), ("top_radius", 0.0), ("height", 6.0)]
        }
        (PrimKind::Cone, "chamfer_ring") => {
            &[("base_radius", 20.0), ("top_radius", 14.0), ("height", 4.0)]
        }
        (PrimKind::Cone, "bullet") => {
            &[("base_radius", 8.0), ("top_radius", 0.0), ("height", 20.0)]
        }
        (PrimKind::Cone, "rocket_nose") => {
            &[("base_radius", 15.0), ("top_radius", 0.0), ("height", 60.0)]
        }
        (PrimKind::Cone, "drill_tip") => {
            &[("base_radius", 10.0), ("top_radius", 0.0), ("height", 9.0)]
        }
        (PrimKind::Cone, "taper_pin") => {
            &[("base_radius", 6.0), ("top_radius", 4.0), ("height", 40.0)]
        }
        (PrimKind::Cone, "spike") => &[("base_radius", 3.0), ("top_radius", 0.0), ("height", 30.0)],
        (PrimKind::Cone, "cup_cone") => {
            &[("base_radius", 5.0), ("top_radius", 25.0), ("height", 20.0)]
        }
        (PrimKind::Cone, "conical_seat") => {
            &[("base_radius", 20.0), ("top_radius", 10.0), ("height", 8.0)]
        }
        (PrimKind::Cone, "diffuser") => {
            &[("base_radius", 8.0), ("top_radius", 25.0), ("height", 35.0)]
        }
        (PrimKind::Cone, "mandrel") => &[
            ("base_radius", 25.0),
            ("top_radius", 18.0),
            ("height", 80.0),
        ],

        // ── Torus ─────────────────────────────────────────────────────────────
        (PrimKind::Torus, "o_ring") => &[("major", 15.0), ("minor", 1.5)],
        (PrimKind::Torus, "gasket") => &[("major", 25.0), ("minor", 5.0)],
        (PrimKind::Torus, "snap_ring") => &[("major", 20.0), ("minor", 2.0)],
        (PrimKind::Torus, "circlip") => &[("major", 12.0), ("minor", 1.5)],
        (PrimKind::Torus, "pipe_elbow") => &[("major", 20.0), ("minor", 8.0)],
        (PrimKind::Torus, "donut") => &[("major", 20.0), ("minor", 10.0)],
        (PrimKind::Torus, "tire_profile") => &[("major", 60.0), ("minor", 20.0)],
        (PrimKind::Torus, "rim_ring") => &[("major", 50.0), ("minor", 4.0)],
        (PrimKind::Torus, "coil_form") => &[("major", 15.0), ("minor", 3.0)],
        (PrimKind::Torus, "anchor_ring") => &[("major", 30.0), ("minor", 12.0)],
        (PrimKind::Torus, "bearing_race") => &[("major", 25.0), ("minor", 5.0)],
        (PrimKind::Torus, "weld_ring") => &[("major", 20.0), ("minor", 3.0)],
        (PrimKind::Torus, "v_seal") => &[("major", 18.0), ("minor", 4.0)],
        (PrimKind::Torus, "crown_ring") => &[("major", 22.0), ("minor", 6.0)],
        (PrimKind::Torus, "bracelet") => &[("major", 30.0), ("minor", 8.0)],

        // ── Pyramid ───────────────────────────────────────────────────────────
        (PrimKind::Pyramid, "square_pyramid") => {
            &[("base_width", 30.0), ("base_depth", 30.0), ("height", 40.0)]
        }
        (PrimKind::Pyramid, "tetrahedron") => {
            &[("base_width", 25.0), ("base_depth", 25.0), ("height", 35.0)]
        }
        (PrimKind::Pyramid, "obelisk") => {
            &[("base_width", 12.0), ("base_depth", 12.0), ("height", 80.0)]
        }
        (PrimKind::Pyramid, "low_pyramid") => {
            &[("base_width", 60.0), ("base_depth", 60.0), ("height", 10.0)]
        }
        (PrimKind::Pyramid, "steep_pyramid") => {
            &[("base_width", 20.0), ("base_depth", 20.0), ("height", 60.0)]
        }
        (PrimKind::Pyramid, "hip_roof") => {
            &[("base_width", 80.0), ("base_depth", 50.0), ("height", 25.0)]
        }
        (PrimKind::Pyramid, "diamond_tip") => {
            &[("base_width", 20.0), ("base_depth", 20.0), ("height", 15.0)]
        }
        (PrimKind::Pyramid, "arrowhead") => {
            &[("base_width", 30.0), ("base_depth", 10.0), ("height", 20.0)]
        }
        (PrimKind::Pyramid, "ramp") => {
            &[("base_width", 60.0), ("base_depth", 30.0), ("height", 20.0)]
        }
        (PrimKind::Pyramid, "chisel") => {
            &[("base_width", 20.0), ("base_depth", 5.0), ("height", 30.0)]
        }
        (PrimKind::Pyramid, "marker") => {
            &[("base_width", 10.0), ("base_depth", 10.0), ("height", 12.0)]
        }
        (PrimKind::Pyramid, "frustum_pyramid") => {
            &[("base_width", 40.0), ("base_depth", 40.0), ("height", 30.0)]
        }
        (PrimKind::Pyramid, "temple") => {
            &[("base_width", 80.0), ("base_depth", 80.0), ("height", 15.0)]
        }
        (PrimKind::Pyramid, "spire") => &[
            ("base_width", 10.0),
            ("base_depth", 10.0),
            ("height", 100.0),
        ],
        (PrimKind::Pyramid, "tent") => {
            &[("base_width", 50.0), ("base_depth", 40.0), ("height", 18.0)]
        }

        // ── Prism ─────────────────────────────────────────────────────────────
        (PrimKind::Prism, "triangle") => {
            &[("sides", 3.0), ("flat_to_flat", 20.0), ("height", 30.0)]
        }
        (PrimKind::Prism, "square") => &[("sides", 4.0), ("flat_to_flat", 20.0), ("height", 30.0)],
        (PrimKind::Prism, "pentagon") => {
            &[("sides", 5.0), ("flat_to_flat", 18.0), ("height", 25.0)]
        }
        (PrimKind::Prism, "hex") => &[("sides", 6.0), ("flat_to_flat", 19.0), ("height", 8.0)],
        (PrimKind::Prism, "heptagon") => {
            &[("sides", 7.0), ("flat_to_flat", 18.0), ("height", 25.0)]
        }
        (PrimKind::Prism, "octagon") => &[("sides", 8.0), ("flat_to_flat", 20.0), ("height", 25.0)],
        (PrimKind::Prism, "decagon") => {
            &[("sides", 10.0), ("flat_to_flat", 20.0), ("height", 25.0)]
        }
        (PrimKind::Prism, "hex_bar") => {
            &[("sides", 6.0), ("flat_to_flat", 19.0), ("height", 120.0)]
        }
        (PrimKind::Prism, "triangle_bar") => {
            &[("sides", 3.0), ("flat_to_flat", 15.0), ("height", 100.0)]
        }
        (PrimKind::Prism, "hex_standoff") => {
            &[("sides", 6.0), ("flat_to_flat", 8.0), ("height", 20.0)]
        }
        (PrimKind::Prism, "hex_socket") => {
            &[("sides", 6.0), ("flat_to_flat", 10.0), ("height", 12.0)]
        }
        (PrimKind::Prism, "spline_approx") => {
            &[("sides", 18.0), ("flat_to_flat", 20.0), ("height", 50.0)]
        }
        (PrimKind::Prism, "diamond_prism") => {
            &[("sides", 4.0), ("flat_to_flat", 14.0), ("height", 30.0)]
        }
        (PrimKind::Prism, "key_prism") => {
            &[("sides", 4.0), ("flat_to_flat", 6.0), ("height", 15.0)]
        }
        (PrimKind::Prism, "allen_key") => {
            &[("sides", 6.0), ("flat_to_flat", 5.0), ("height", 60.0)]
        }

        // ── Gear ──────────────────────────────────────────────────────────────
        (PrimKind::Gear, "spur_8t") => &[
            ("teeth", 8.0),
            ("radius", 15.0),
            ("tooth_height", 4.0),
            ("tooth_fraction", 0.5),
            ("height", 10.0),
        ],
        (PrimKind::Gear, "spur_12t") => &[
            ("teeth", 12.0),
            ("radius", 25.0),
            ("tooth_height", 5.0),
            ("tooth_fraction", 0.5),
            ("height", 12.0),
        ],
        (PrimKind::Gear, "spur_20t") => &[
            ("teeth", 20.0),
            ("radius", 40.0),
            ("tooth_height", 6.0),
            ("tooth_fraction", 0.5),
            ("height", 14.0),
        ],
        (PrimKind::Gear, "spur_36t") => &[
            ("teeth", 36.0),
            ("radius", 70.0),
            ("tooth_height", 7.0),
            ("tooth_fraction", 0.5),
            ("height", 16.0),
        ],
        (PrimKind::Gear, "spur_60t") => &[
            ("teeth", 60.0),
            ("radius", 110.0),
            ("tooth_height", 8.0),
            ("tooth_fraction", 0.5),
            ("height", 18.0),
        ],
        (PrimKind::Gear, "ratchet_24t") => &[
            ("teeth", 24.0),
            ("radius", 30.0),
            ("tooth_height", 4.0),
            ("tooth_fraction", 0.35),
            ("height", 8.0),
        ],
        (PrimKind::Gear, "ratchet_48t") => &[
            ("teeth", 48.0),
            ("radius", 40.0),
            ("tooth_height", 3.0),
            ("tooth_fraction", 0.3),
            ("height", 8.0),
        ],
        (PrimKind::Gear, "escape_30t") => &[
            ("teeth", 30.0),
            ("radius", 25.0),
            ("tooth_height", 3.0),
            ("tooth_fraction", 0.25),
            ("height", 4.0),
        ],
        (PrimKind::Gear, "sprocket_9t") => &[
            ("teeth", 9.0),
            ("radius", 18.0),
            ("tooth_height", 4.0),
            ("tooth_fraction", 0.45),
            ("height", 8.0),
        ],
        (PrimKind::Gear, "timing_20t") => &[
            ("teeth", 20.0),
            ("radius", 20.0),
            ("tooth_height", 3.0),
            ("tooth_fraction", 0.4),
            ("height", 10.0),
        ],
        (PrimKind::Gear, "sun_12t") => &[
            ("teeth", 12.0),
            ("radius", 15.0),
            ("tooth_height", 4.0),
            ("tooth_fraction", 0.5),
            ("height", 12.0),
        ],
        (PrimKind::Gear, "planet_18t") => &[
            ("teeth", 18.0),
            ("radius", 22.0),
            ("tooth_height", 4.0),
            ("tooth_fraction", 0.5),
            ("height", 12.0),
        ],
        (PrimKind::Gear, "ring_60t") => &[
            ("teeth", 60.0),
            ("radius", 80.0),
            ("tooth_height", 6.0),
            ("tooth_fraction", 0.5),
            ("height", 15.0),
        ],
        (PrimKind::Gear, "crown_24t") => &[
            ("teeth", 24.0),
            ("radius", 35.0),
            ("tooth_height", 5.0),
            ("tooth_fraction", 0.5),
            ("height", 12.0),
        ],
        (PrimKind::Gear, "worm_wheel_30t") => &[
            ("teeth", 30.0),
            ("radius", 45.0),
            ("tooth_height", 8.0),
            ("tooth_fraction", 0.5),
            ("height", 20.0),
        ],

        _ => &[],
    }
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Evaluate a `define` block into an `SdfNode` plus a bounds hint (mm).
///
/// `parent_scale` is the unit scale inherited from an outer scope (usually 1.0
/// for mm, or the scale from a wrapping `assemble` block).
pub fn eval_define(block: &DefineBlock, parent_scale: f32) -> Result<(SdfNode, f32)> {
    let scale = block
        .items
        .iter()
        .find_map(|i| {
            if let DefineItem::Units(u) = i {
                Some(unit_scale(*u))
            } else {
                None
            }
        })
        .unwrap_or(parent_scale);

    // Collect all primitive nodes.
    let mut prims: Vec<(SdfNode, f32)> = Vec::new();
    for item in &block.items {
        if let DefineItem::Primitive(p) = item {
            prims.push(eval_primitive(p, scale)?);
        }
    }
    if prims.is_empty() {
        return Err(PolyhedraError::GeometryError {
            message: format!(
                "define '{}' has no primitive — expected cube/sphere/cylinder/…",
                block.name
            ),
        });
    }

    // Union all primitives together.
    let (mut node, mut bounds) = prims.remove(0);
    for (n, b) in prims {
        bounds = bounds.max(b);
        node = Box::new(UnionNode { a: node, b: n });
    }

    // Apply block-level manipulations.
    for item in &block.items {
        if let DefineItem::Manip(m) = item {
            (node, bounds) = apply_manip(node, bounds, m, scale);
        }
    }

    Ok((node, bounds))
}

/// Evaluate an `assemble` block.
///
/// `defines` is a name → `&DefineBlock` map for all blocks in the file.
pub fn eval_assemble(
    block: &AssembleBlock,
    defines: &HashMap<String, &DefineBlock>,
    parent_scale: f32,
) -> Result<(SdfNode, f32)> {
    let scale = block
        .items
        .iter()
        .find_map(|i| {
            if let AssembleItem::Units(u) = i {
                Some(unit_scale(*u))
            } else {
                None
            }
        })
        .unwrap_or(parent_scale);

    let mut result: Option<(SdfNode, f32)> = None;

    for item in &block.items {
        let AssembleItem::Op(op) = item else { continue };

        let name = match op {
            AssembleOp::Place { name, .. } => name,
            AssembleOp::Cut { name, .. } => name,
            AssembleOp::Join { name, .. } => name,
            AssembleOp::Intersect { name, .. } => name,
            AssembleOp::Subtract { name, .. } => name,
        };

        let def = defines
            .get(name.as_str())
            .ok_or_else(|| PolyhedraError::GeometryError {
                message: format!("assemble '{}': object '{name}' is not defined", block.name),
            })?;

        let (mut node, b) = eval_define(def, scale)?;

        // Apply translation.
        let at = match op {
            AssembleOp::Place { at, .. } => at,
            AssembleOp::Cut { at, .. } => at,
            AssembleOp::Join { at, .. } => at,
            AssembleOp::Intersect { at, .. } => at,
            AssembleOp::Subtract { at, .. } => at,
        };
        if let Position::Coords(x, y, z) = at {
            let offset = Vec3::new(*x, *y, *z) * scale;
            if offset != Vec3::ZERO {
                node = Box::new(Translate {
                    inner: node,
                    offset,
                });
            }
        }

        let bounds = b + match at {
            Position::Origin => 0.0,
            Position::Coords(x, y, z) => Vec3::new(*x * scale, *y * scale, *z * scale).length(),
        };

        result = Some(match result {
            None => (node, bounds),
            Some((acc, ab)) => {
                let new_bounds = ab.max(bounds);
                let combined: SdfNode = match op {
                    AssembleOp::Place { .. } | AssembleOp::Join { .. } => {
                        Box::new(UnionNode { a: acc, b: node })
                    }
                    AssembleOp::Cut { .. } | AssembleOp::Subtract { .. } => {
                        Box::new(DifferenceNode { a: acc, b: node })
                    }
                    AssembleOp::Intersect { .. } => Box::new(IntersectionNode { a: acc, b: node }),
                };
                (combined, new_bounds)
            }
        });
    }

    // Block-level manipulations on the assembled result.
    if let Some((mut node, mut bounds)) = result {
        for item in &block.items {
            if let AssembleItem::Manip(m) = item {
                (node, bounds) = apply_manip(node, bounds, m, scale);
            }
        }
        Ok((node, bounds))
    } else {
        Err(PolyhedraError::GeometryError {
            message: format!("assemble '{}' has no operations", block.name),
        })
    }
}

/// Convenience: parse `path` and evaluate the first `define` block.
pub fn eval_file_first_define(path: impl AsRef<std::path::Path>) -> Result<(SdfNode, f32)> {
    let file = super::parse_file(path)?;
    let def = file
        .defines()
        .next()
        .ok_or_else(|| PolyhedraError::GeometryError {
            message: "file contains no 'define' block".into(),
        })?;
    eval_define(def, 1.0)
}

/// Convenience: parse `path` and evaluate the first `assemble` block.
pub fn eval_file_first_assemble(path: impl AsRef<std::path::Path>) -> Result<(SdfNode, f32)> {
    let file = super::parse_file(path)?;
    let defines: HashMap<String, &DefineBlock> =
        file.defines().map(|d| (d.name.clone(), d)).collect();
    let asm = file
        .assemblies()
        .next()
        .ok_or_else(|| PolyhedraError::GeometryError {
            message: "file contains no 'assemble' block".into(),
        })?;
    eval_assemble(asm, &defines, 1.0)
}

// ── Primitive builder ──────────────────────────────────────────────────────────

fn eval_primitive(p: &PrimitiveBlock, parent_scale: f32) -> Result<(SdfNode, f32)> {
    let scale = p.units.map(unit_scale).unwrap_or(parent_scale);
    let style_defs = p
        .style
        .as_deref()
        .map(|s| style_defaults(p.kind, s))
        .unwrap_or(&[]);

    let prop = |key: &str, fallback: f32| -> f32 {
        p.props
            .iter()
            .find(|pr| pr.key == key)
            .map(|pr| pr.value * scale)
            .or_else(|| {
                style_defs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v * scale)
            })
            .unwrap_or(fallback * scale)
    };
    let prop2 = |k1: &str, k2: &str, fb: f32| -> f32 {
        p.props
            .iter()
            .find(|pr| pr.key == k1 || pr.key == k2)
            .map(|pr| pr.value * scale)
            .or_else(|| {
                style_defs
                    .iter()
                    .find(|(k, _)| *k == k1 || *k == k2)
                    .map(|(_, v)| v * scale)
            })
            .unwrap_or(fb * scale)
    };

    let (node, bounds): (SdfNode, f32) = match p.kind {
        PrimKind::Cube => {
            let w = prop("width", 10.0);
            let d = prop("depth", w / scale);
            let h = prop("height", w / scale);
            let node: SdfNode = Box::new(CubeSdf::new(w, d, h));
            (node, w.max(d).max(h) * 0.56)
        }
        PrimKind::Sphere => {
            let r = prop("radius", 5.0);
            (Box::new(SphereSdf::new(r)), r * 1.1)
        }
        PrimKind::Cylinder => {
            let r = prop("radius", 5.0);
            let h = prop("height", 10.0);
            (Box::new(CylinderSdf::new(r, h)), r.max(h * 0.5) * 1.1)
        }
        PrimKind::Cone => {
            let br = prop2("base_radius", "radius", 5.0);
            let tr = prop2("top_radius", "top", 0.0);
            let h = prop("height", 10.0);
            (Box::new(ConeSdf::new(br, tr, h)), br.max(h * 0.5) * 1.1)
        }
        PrimKind::Torus => {
            let major = prop2("major", "major_radius", 10.0);
            let minor = prop2("minor", "minor_radius", 2.0);
            (Box::new(TorusSdf::new(major, minor)), (major + minor) * 1.1)
        }
        PrimKind::Pyramid => {
            let bw = prop2("base_width", "base", 10.0);
            let bd = prop("base_depth", bw / scale);
            let h = prop("height", 10.0);
            (
                Box::new(PyramidSdf::new(bw, bd, h)),
                bw.max(bd).max(h) * 0.6,
            )
        }
        PrimKind::Prism => {
            let sides = p
                .props
                .iter()
                .find(|pr| pr.key == "sides" || pr.key == "n")
                .map(|pr| pr.value as u32)
                .or_else(|| {
                    style_defs
                        .iter()
                        .find(|(k, _)| *k == "sides")
                        .map(|(_, v)| *v as u32)
                })
                .unwrap_or(6);
            let ftf = prop2("flat_to_flat", "radius", 10.0);
            let h = prop("height", 10.0);
            (Box::new(PrismSdf::new(sides, ftf, h)), ftf.max(h) * 0.56)
        }
        PrimKind::Gear => {
            let teeth = p
                .props
                .iter()
                .find(|pr| pr.key == "teeth" || pr.key == "n")
                .map(|pr| pr.value as u32)
                .or_else(|| {
                    style_defs
                        .iter()
                        .find(|(k, _)| *k == "teeth")
                        .map(|(_, v)| *v as u32)
                })
                .unwrap_or(12);
            let pitch_r = prop2("pitch_radius", "radius", 25.0);
            let tooth_h = prop("tooth_height", 5.0);
            let tooth_f = p
                .props
                .iter()
                .find(|pr| pr.key == "tooth_fraction")
                .map(|pr| pr.value)
                .or_else(|| {
                    style_defs
                        .iter()
                        .find(|(k, _)| *k == "tooth_fraction")
                        .map(|(_, v)| *v)
                })
                .unwrap_or(0.5);
            let h = prop("height", 12.0);
            let tip_r = pitch_r + tooth_h * 0.5;
            (
                Box::new(GearSdf::new(teeth, pitch_r, tooth_h, tooth_f, h)),
                tip_r.max(h * 0.5) * 1.1,
            )
        }
        PrimKind::Thread => {
            let r = prop("outer_radius", 10.0);
            let pitch = prop("pitch", 2.0);
            let h = prop("height", 30.0);
            (Box::new(ThreadSdf::new(r, pitch, h)), r.max(h * 0.5) * 1.1)
        }
        PrimKind::Spring => {
            let coil_r = prop("coil_radius", 15.0);
            let wire_r = prop("wire_radius", 2.0);
            let pitch = prop("pitch", 8.0);
            let turns = prop("turns", 5.0);
            let h = pitch * turns;
            (
                Box::new(SpringSdf::new(coil_r, wire_r, pitch, turns)),
                (coil_r + wire_r).max(h * 0.5) * 1.1,
            )
        }
        PrimKind::Knurl => {
            let r = prop("radius", 15.0);
            let h = prop("height", 30.0);
            let bump_depth = prop("bump_depth", 0.5);
            let n_rows = p
                .props
                .iter()
                .find(|pr| pr.key == "rows")
                .map(|pr| pr.value as u32)
                .or_else(|| {
                    style_defs
                        .iter()
                        .find(|(k, _)| *k == "rows")
                        .map(|(_, v)| *v as u32)
                })
                .unwrap_or(24);
            let pitch = prop("pitch", 3.0);
            (
                Box::new(KnurlSdf::new(r, h, bump_depth, n_rows, pitch)),
                (r + bump_depth).max(h * 0.5) * 1.1,
            )
        }
        PrimKind::Spline => {
            let pitch_r = prop("pitch_radius", 15.0);
            let tooth_h = prop("tooth_height", 3.0);
            let h = prop("height", 40.0);
            let n_splines = p
                .props
                .iter()
                .find(|pr| pr.key == "sides" || pr.key == "n_splines")
                .map(|pr| pr.value as u32)
                .or_else(|| {
                    style_defs
                        .iter()
                        .find(|(k, _)| *k == "sides")
                        .map(|(_, v)| *v as u32)
                })
                .unwrap_or(12);
            let tooth_f = p
                .props
                .iter()
                .find(|pr| pr.key == "tooth_fraction")
                .map(|pr| pr.value)
                .or_else(|| {
                    style_defs
                        .iter()
                        .find(|(k, _)| *k == "tooth_fraction")
                        .map(|(_, v)| *v)
                })
                .unwrap_or(0.5);
            let tip_r = pitch_r + tooth_h;
            (
                Box::new(SplineSdf::new(pitch_r, tooth_h, h, n_splines, tooth_f)),
                tip_r.max(h * 0.5) * 1.1,
            )
        }
        PrimKind::IBeam => {
            let flange_w = prop("flange_width", 50.0);
            let flange_t = prop("flange_thickness", 8.0);
            let web_h = prop("web_height", 80.0);
            let web_t = prop("web_thickness", 6.0);
            let l = prop("length", 200.0);
            let half_section_h = web_h * 0.5 + flange_t;
            (
                Box::new(IBeamSdf::new(flange_w, flange_t, web_h, web_t, l)),
                flange_w.max(half_section_h * 2.0).max(l) * 0.56,
            )
        }
        PrimKind::TSlot => {
            let side = prop("side", 40.0);
            let slot_w = prop("slot_width", 8.0);
            let slot_hw = prop("slot_head_width", 16.0);
            let slot_d = prop("slot_depth", 8.0);
            let l = prop("length", 100.0);
            (
                Box::new(TSlotSdf::new(side, slot_w, slot_hw, slot_d, l)),
                side.max(l) * 0.6,
            )
        }
        PrimKind::Rack => {
            let length = prop("length", 100.0);
            let width = prop("width", 20.0);
            let height = prop("height", 15.0);
            let tooth_h = prop("tooth_height", 5.0);
            let pitch = prop("pitch", 8.0);
            let tooth_f = p
                .props
                .iter()
                .find(|pr| pr.key == "tooth_fraction")
                .map(|pr| pr.value)
                .or_else(|| {
                    style_defs
                        .iter()
                        .find(|(k, _)| *k == "tooth_fraction")
                        .map(|(_, v)| *v)
                })
                .unwrap_or(0.6);
            (
                Box::new(RackSdf::new(length, width, height, tooth_h, pitch, tooth_f)),
                length.max(width + tooth_h).max(height) * 0.56,
            )
        }
        PrimKind::Sprocket => {
            let pitch_r = prop("pitch_radius", 30.0);
            let tooth_h = prop("tooth_height", 5.0);
            let bore_r = prop("bore_radius", 8.0);
            let h = prop("height", 10.0);
            let n_teeth = p
                .props
                .iter()
                .find(|pr| pr.key == "teeth")
                .map(|pr| pr.value as u32)
                .or_else(|| {
                    style_defs
                        .iter()
                        .find(|(k, _)| *k == "teeth")
                        .map(|(_, v)| *v as u32)
                })
                .unwrap_or(16);
            let tip_r = pitch_r + tooth_h;
            (
                Box::new(SprocketSdf::new(pitch_r, tooth_h, bore_r, h, n_teeth)),
                tip_r.max(h * 0.5) * 1.1,
            )
        }
        PrimKind::Bearing => {
            let outer_r = prop("outer_radius", 30.0);
            let inner_r = prop("inner_radius", 12.0);
            let h = prop("height", 10.0);
            let n_balls = p
                .props
                .iter()
                .find(|pr| pr.key == "balls")
                .map(|pr| pr.value as u32)
                .or_else(|| {
                    style_defs
                        .iter()
                        .find(|(k, _)| *k == "balls")
                        .map(|(_, v)| *v as u32)
                })
                .unwrap_or(8);
            (
                Box::new(BearingSdf::new(outer_r, inner_r, h, n_balls)),
                outer_r.max(h * 0.5) * 1.1,
            )
        }
        PrimKind::Cam => {
            let cam_r = prop("cam_radius", 20.0);
            let ecc = prop("eccentricity", 8.0);
            let h = prop("height", 12.0);
            (
                Box::new(CamSdf::new(cam_r, ecc, h)),
                (cam_r + ecc).max(h * 0.5) * 1.1,
            )
        }
        PrimKind::Dovetail => {
            let top_w = prop("top_width", 20.0);
            let bot_w = prop("bottom_width", 30.0);
            let profile_h = prop("profile_height", 15.0);
            let l = prop("length", 80.0);
            (
                Box::new(DovetailSdf::new(top_w, bot_w, profile_h, l)),
                bot_w.max(profile_h).max(l) * 0.56,
            )
        }
        PrimKind::CskHole => {
            let bore_d = prop("bore_diameter", 6.0);
            let csk_d = prop("csk_diameter", 12.0);
            let csk_depth = prop("csk_depth", 4.0);
            let total_d = prop("total_depth", 20.0);
            (
                Box::new(CskHoleSdf::new(bore_d, csk_d, csk_depth, total_d)),
                csk_d.max(total_d) * 0.6,
            )
        }
        PrimKind::HexBolt => {
            let af = prop("across_flats", 17.0);
            let head_h = prop("head_height", 10.0);
            let shank_d = prop("shank_diameter", 10.0);
            let shank_l = prop("shank_length", 30.0);
            let total_l = shank_l + head_h;
            (
                Box::new(HexBoltSdf::new(af, head_h, shank_d, shank_l)),
                af.max(total_l) * 0.6,
            )
        }
        PrimKind::Star => {
            let outer_r = prop("outer_radius", 20.0);
            let inner_r = prop("inner_radius", 10.0);
            let n_points = p
                .props
                .iter()
                .find(|pr| pr.key == "points")
                .map(|pr| pr.value as u32)
                .or_else(|| {
                    style_defs
                        .iter()
                        .find(|(k, _)| *k == "points")
                        .map(|(_, v)| *v as u32)
                })
                .unwrap_or(5);
            let h = prop("height", 10.0);
            (
                Box::new(StarSdf::new(outer_r, inner_r, n_points, h)),
                outer_r.max(h * 0.5) * 1.1,
            )
        }
        PrimKind::Cross => {
            let arm_w = prop("arm_width", 10.0);
            let arm_l = prop("arm_length", 40.0);
            let h = prop("height", 10.0);
            (
                Box::new(CrossSectionSdf::new(arm_w, arm_l, h)),
                arm_l.max(h) * 0.56,
            )
        }
    };

    // Apply per-primitive move statements.
    let mut node = node;
    for mv in &p.moves {
        let v = mv.value * scale;
        let offset = match mv.axis {
            Axis::X => Vec3::new(v, 0.0, 0.0),
            Axis::Y => Vec3::new(0.0, v, 0.0),
            Axis::Z => Vec3::new(0.0, 0.0, v),
        };
        node = Box::new(Translate {
            inner: node,
            offset,
        });
    }

    Ok((node, bounds))
}

// ── Manipulation applier ───────────────────────────────────────────────────────

fn apply_manip(node: SdfNode, bounds: f32, m: &Manipulation, scale: f32) -> (SdfNode, f32) {
    match m {
        Manipulation::Chamfer(r) | Manipulation::Fillet(r) => {
            let r = r * scale;
            (
                Box::new(OffsetNode {
                    inner: node,
                    offset: r,
                }),
                bounds + r,
            )
        }
        Manipulation::Shell(t) => {
            let t = t * scale;
            (
                Box::new(ShellNode {
                    inner: node,
                    thickness: t,
                }),
                bounds,
            )
        }
        Manipulation::Hole { diameter, .. } => {
            // Subtract a tall cylinder through the centre of the shape.
            let r = (diameter * scale) * 0.5;
            let h = bounds * 4.0; // tall enough to pierce any reasonable shape
            let bore: SdfNode = Box::new(CylinderSdf::new(r, h));
            (Box::new(DifferenceNode { a: node, b: bore }), bounds)
        }
        Manipulation::Thread { .. } | Manipulation::Pattern { .. } => {
            // Not yet implemented at the SDF level; pass through unchanged.
            (node, bounds)
        }
    }
}
