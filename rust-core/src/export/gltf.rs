//! Binary GLTF 2.0 (GLB) exporter.
//!
//! Produces a single self-contained `.glb` file accepted by Blender, Three.js,
//! Babylon.js, Godot, Unity (via importer), and the Khronos GLTF Validator.
//!
//! # GLB structure
//! ```text
//! [12 bytes] Header: magic(glTF), version(2), totalLength
//! [JSON chunk]: chunkLength + chunkType(JSON) + JSON UTF-8 (padded with 0x20)
//! [BIN  chunk]: chunkLength + chunkType(BIN\0) + binary data (padded with 0x00)
//! ```
//!
//! # Binary buffer layout
//! 1. **Index buffer** — triangle count × 3 × `uint32` (ELEMENT_ARRAY_BUFFER)
//! 2. **Position buffer** — vertex count × `vec3<f32>`  (ARRAY_BUFFER)
//! 3. **Normal buffer**   — vertex count × `vec3<f32>`  (ARRAY_BUFFER)

use std::io::Write;

use glam::Vec3;
use serde_json::json;

use crate::error::Result;
use crate::mesher::Mesh;

use super::compute_vertex_normals;

// ── GLB constants ──────────────────────────────────────────────────────────

const GLB_MAGIC: u32 = 0x4654_6C67; // b"glTF" — 0x67='g',0x6C='l',0x54='T',0x46='F' LE
const GLB_VERSION: u32 = 2;
const CHUNK_JSON: u32 = 0x4E4F_534A; // b"JSON"
const CHUNK_BIN: u32 = 0x004E_4942; // b"BIN\0"
const TARGET_INDEX: u32 = 34963; // ELEMENT_ARRAY_BUFFER
const TARGET_VERTEX: u32 = 34962; // ARRAY_BUFFER
const COMP_UINT32: u32 = 5125;
const COMP_FLOAT: u32 = 5126;

/// Serialize `mesh` to binary GLB bytes.
pub fn to_bytes(mesh: &Mesh) -> Result<Vec<u8>> {
    let nv = mesh.vertex_count();
    let ni = mesh.triangle_count() * 3; // total index count
    let normals = compute_vertex_normals(&mesh.vertices, &mesh.triangles);

    // ── Binary buffer ────────────────────────────────────────────────────────
    let idx_bytes = (ni * 4) as u32;
    let pos_bytes = (nv * 12) as u32; // 3 × f32
    let norm_bytes = (nv * 12) as u32;
    let bin_len = idx_bytes + pos_bytes + norm_bytes;

    let mut bin: Vec<u8> = Vec::with_capacity(bin_len as usize);

    // 1. Indices (uint32 LE)
    for &[a, b, c] in &mesh.triangles {
        bin.write_all(&a.to_le_bytes())?;
        bin.write_all(&b.to_le_bytes())?;
        bin.write_all(&c.to_le_bytes())?;
    }

    // 2. Positions (f32 LE)
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for &v in &mesh.vertices {
        min = min.min(v);
        max = max.max(v);
        bin.write_all(&v.x.to_le_bytes())?;
        bin.write_all(&v.y.to_le_bytes())?;
        bin.write_all(&v.z.to_le_bytes())?;
    }

    // 3. Normals (f32 LE)
    for n in &normals {
        bin.write_all(&n.x.to_le_bytes())?;
        bin.write_all(&n.y.to_le_bytes())?;
        bin.write_all(&n.z.to_le_bytes())?;
    }

    // Pad binary buffer to 4-byte boundary.
    let bin_padded_len = pad4(bin.len());
    bin.resize(bin_padded_len, 0u8);

    // ── GLTF JSON ────────────────────────────────────────────────────────────
    let json_val = json!({
        "asset": {
            "version": "2.0",
            "generator": "polyhedra"
        },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes":  [{ "mesh": 0 }],
        "meshes": [{
            "name": "mesh",
            "primitives": [{
                "attributes": { "POSITION": 1, "NORMAL": 2 },
                "indices": 0,
                "mode": 4
            }]
        }],
        "buffers": [{
            "byteLength": bin_padded_len
        }],
        "bufferViews": [
            // Index buffer view
            {
                "buffer": 0, "byteOffset": 0,
                "byteLength": idx_bytes, "target": TARGET_INDEX
            },
            // Position buffer view
            {
                "buffer": 0, "byteOffset": idx_bytes,
                "byteLength": pos_bytes, "target": TARGET_VERTEX
            },
            // Normal buffer view
            {
                "buffer": 0, "byteOffset": idx_bytes + pos_bytes,
                "byteLength": norm_bytes, "target": TARGET_VERTEX
            }
        ],
        "accessors": [
            // Accessor 0 — indices (SCALAR uint32)
            {
                "bufferView": 0, "byteOffset": 0,
                "componentType": COMP_UINT32,
                "count": ni, "type": "SCALAR"
            },
            // Accessor 1 — positions (VEC3 float)
            {
                "bufferView": 1, "byteOffset": 0,
                "componentType": COMP_FLOAT,
                "count": nv, "type": "VEC3",
                "min": [min.x, min.y, min.z],
                "max": [max.x, max.y, max.z]
            },
            // Accessor 2 — normals (VEC3 float)
            {
                "bufferView": 2, "byteOffset": 0,
                "componentType": COMP_FLOAT,
                "count": nv, "type": "VEC3"
            }
        ]
    });

    let json_str = serde_json::to_string(&json_val).map_err(|e| {
        crate::error::PolyhedraError::ExportError {
            path: String::from("<gltf json>"),
            message: e.to_string(),
        }
    })?;

    // Pad JSON to 4-byte boundary with spaces (GLB spec requirement).
    let json_padded_len = pad4(json_str.len());
    let mut json_bytes = json_str.into_bytes();
    json_bytes.resize(json_padded_len, b' ');

    // ── Assemble GLB ─────────────────────────────────────────────────────────
    let total_len = 12                                      // GLB header
        + 8 + json_bytes.len()                  // JSON chunk header + data
        + 8 + bin_padded_len; // BIN  chunk header + data

    let mut glb: Vec<u8> = Vec::with_capacity(total_len);

    // GLB header
    glb.write_all(&GLB_MAGIC.to_le_bytes())?;
    glb.write_all(&GLB_VERSION.to_le_bytes())?;
    glb.write_all(&(total_len as u32).to_le_bytes())?;

    // JSON chunk
    glb.write_all(&(json_bytes.len() as u32).to_le_bytes())?;
    glb.write_all(&CHUNK_JSON.to_le_bytes())?;
    glb.write_all(&json_bytes)?;

    // BIN chunk
    glb.write_all(&(bin_padded_len as u32).to_le_bytes())?;
    glb.write_all(&CHUNK_BIN.to_le_bytes())?;
    glb.write_all(&bin)?;

    Ok(glb)
}

/// Round `n` up to the nearest multiple of 4.
#[inline]
fn pad4(n: usize) -> usize {
    (n + 3) & !3
}
