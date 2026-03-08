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
use crate::sdf::operations::{DifferenceNode, IntersectionNode, UnionNode};
use crate::sdf::transform::Translate;
use crate::sdf::SdfNode;
use crate::sdf::primitives::{
    ConeSdf, CubeSdf, CylinderSdf, PrismSdf, PyramidSdf, SphereSdf, TorusSdf,
};

use super::ast::*;

// ── Unit scale ─────────────────────────────────────────────────────────────────

fn unit_scale(u: Unit) -> f32 {
    u.to_mm()
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Evaluate a `define` block into an `SdfNode` plus a bounds hint (mm).
///
/// `parent_scale` is the unit scale inherited from an outer scope (usually 1.0
/// for mm, or the scale from a wrapping `assemble` block).
pub fn eval_define(block: &DefineBlock, parent_scale: f32) -> Result<(SdfNode, f32)> {
    let scale = block.items.iter().find_map(|i| {
        if let DefineItem::Units(u) = i { Some(unit_scale(*u)) } else { None }
    }).unwrap_or(parent_scale);

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
    let scale = block.items.iter().find_map(|i| {
        if let AssembleItem::Units(u) = i { Some(unit_scale(*u)) } else { None }
    }).unwrap_or(parent_scale);

    let mut result: Option<(SdfNode, f32)> = None;

    for item in &block.items {
        let AssembleItem::Op(op) = item else { continue };

        let name = match op {
            AssembleOp::Place { name, .. }     => name,
            AssembleOp::Cut   { name, .. }     => name,
            AssembleOp::Join      { name, .. } => name,
            AssembleOp::Intersect { name, .. } => name,
            AssembleOp::Subtract  { name, .. } => name,
        };

        let def = defines.get(name.as_str()).ok_or_else(|| {
            PolyhedraError::GeometryError {
                message: format!(
                    "assemble '{}': object '{name}' is not defined",
                    block.name
                ),
            }
        })?;

        let (mut node, b) = eval_define(def, scale)?;

        // Apply translation.
        let at = match op {
            AssembleOp::Place     { at, .. } => at,
            AssembleOp::Cut       { at, .. } => at,
            AssembleOp::Join      { at, .. } => at,
            AssembleOp::Intersect { at, .. } => at,
            AssembleOp::Subtract  { at, .. } => at,
        };
        if let Position::Coords(x, y, z) = at {
            let offset = Vec3::new(*x, *y, *z) * scale;
            if offset != Vec3::ZERO {
                node = Box::new(Translate { inner: node, offset });
            }
        }

        let bounds = b + match at {
            Position::Origin => 0.0,
            Position::Coords(x, y, z) => {
                Vec3::new(*x * scale, *y * scale, *z * scale).length()
            }
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
                    AssembleOp::Intersect { .. } => {
                        Box::new(IntersectionNode { a: acc, b: node })
                    }
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
pub fn eval_file_first_define(
    path: impl AsRef<std::path::Path>,
) -> Result<(SdfNode, f32)> {
    let file = super::parse_file(path)?;
    let def  = file.defines().next().ok_or_else(|| {
        PolyhedraError::GeometryError {
            message: "file contains no 'define' block".into(),
        }
    })?;
    eval_define(def, 1.0)
}

/// Convenience: parse `path` and evaluate the first `assemble` block.
pub fn eval_file_first_assemble(
    path: impl AsRef<std::path::Path>,
) -> Result<(SdfNode, f32)> {
    let file = super::parse_file(path)?;
    let defines: HashMap<String, &DefineBlock> = file
        .defines()
        .map(|d| (d.name.clone(), d))
        .collect();
    let asm = file.assemblies().next().ok_or_else(|| {
        PolyhedraError::GeometryError {
            message: "file contains no 'assemble' block".into(),
        }
    })?;
    eval_assemble(asm, &defines, 1.0)
}

// ── Primitive builder ──────────────────────────────────────────────────────────

fn eval_primitive(p: &PrimitiveBlock, parent_scale: f32) -> Result<(SdfNode, f32)> {
    let scale = p.units.map(unit_scale).unwrap_or(parent_scale);
    let prop  = |key: &str, fallback: f32| -> f32 {
        p.props.iter()
            .find(|pr| pr.key == key)
            .map(|pr| pr.value * scale)
            .unwrap_or(fallback * scale)
    };
    let prop2 = |k1: &str, k2: &str, fb: f32| -> f32 {
        p.props.iter()
            .find(|pr| pr.key == k1 || pr.key == k2)
            .map(|pr| pr.value * scale)
            .unwrap_or(fb * scale)
    };

    let (node, bounds): (SdfNode, f32) = match p.kind {
        PrimKind::Cube => {
            let w = prop("width",  10.0);
            let d = prop("depth",  w / scale);
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
            let tr = prop2("top_radius",  "top",    0.0);
            let h  = prop("height", 10.0);
            (Box::new(ConeSdf::new(br, tr, h)), br.max(h * 0.5) * 1.1)
        }
        PrimKind::Torus => {
            let major = prop2("major", "major_radius", 10.0);
            let minor = prop2("minor", "minor_radius",  2.0);
            (Box::new(TorusSdf::new(major, minor)), (major + minor) * 1.1)
        }
        PrimKind::Pyramid => {
            let bw = prop2("base_width",  "base",  10.0);
            let bd = prop("base_depth",   bw / scale);
            let h  = prop("height", 10.0);
            (Box::new(PyramidSdf::new(bw, bd, h)), bw.max(bd).max(h) * 0.6)
        }
        PrimKind::Prism => {
            let sides = p.props.iter()
                .find(|pr| pr.key == "sides" || pr.key == "n")
                .map(|pr| pr.value as u32)
                .unwrap_or(6);
            let ftf   = prop2("flat_to_flat", "radius", 10.0);
            let h     = prop("height", 10.0);
            (Box::new(PrismSdf::new(sides, ftf, h)), ftf.max(h) * 0.56)
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
        node = Box::new(Translate { inner: node, offset });
    }

    Ok((node, bounds))
}

// ── Manipulation applier ───────────────────────────────────────────────────────

fn apply_manip(
    node:   SdfNode,
    bounds: f32,
    m:      &Manipulation,
    scale:  f32,
) -> (SdfNode, f32) {
    match m {
        Manipulation::Chamfer(r) | Manipulation::Fillet(r) => {
            let r = r * scale;
            (Box::new(OffsetNode { inner: node, offset: r }), bounds + r)
        }
        Manipulation::Shell(t) => {
            let t = t * scale;
            (Box::new(ShellNode { inner: node, thickness: t }), bounds)
        }
        Manipulation::Hole { diameter, .. } => {
            // Approximate: shell with half the diameter as thickness.
            let t = (diameter * scale) * 0.5;
            (Box::new(ShellNode { inner: node, thickness: t }), bounds)
        }
        Manipulation::Thread { .. } | Manipulation::Pattern { .. } => {
            // Not yet implemented at the SDF level; pass through unchanged.
            (node, bounds)
        }
    }
}
