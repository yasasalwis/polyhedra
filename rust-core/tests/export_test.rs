//! Integration tests for the export module.
//!
//! Tests verify byte-level structure of each format without requiring
//! an external mesh viewer — magic numbers, header fields, and size
//! calculations are all deterministic.

use _core::export::{ExportFormat, to_bytes};
use _core::mesher::{MeshConfig, mesh};
use _core::sdf::primitives::{CubeSdf, SphereSdf};

// ── Test fixtures ─────────────────────────────────────────────────────────────

fn sphere_mesh() -> _core::mesher::Mesh {
    mesh(&SphereSdf::new(5.0), &MeshConfig::centered(7.0, 24))
}

fn cube_mesh() -> _core::mesher::Mesh {
    mesh(
        &CubeSdf::new(10.0, 10.0, 10.0),
        &MeshConfig::centered(8.0, 20),
    )
}

// ── ExportFormat ──────────────────────────────────────────────────────────────

#[test]
fn format_from_str_all_variants() {
    use std::str::FromStr;
    assert_eq!(ExportFormat::from_str("stl").unwrap(), ExportFormat::Stl);
    assert_eq!(ExportFormat::from_str("STL").unwrap(), ExportFormat::Stl);
    assert_eq!(ExportFormat::from_str("obj").unwrap(), ExportFormat::Obj);
    assert_eq!(ExportFormat::from_str("ply").unwrap(), ExportFormat::Ply);
    assert_eq!(ExportFormat::from_str("glb").unwrap(), ExportFormat::Glb);
    assert_eq!(ExportFormat::from_str("gltf").unwrap(), ExportFormat::Glb);
}

#[test]
fn format_from_str_unknown_is_err() {
    use std::str::FromStr;
    assert!(ExportFormat::from_str("fbx").is_err());
    assert!(ExportFormat::from_str("").is_err());
}

#[test]
fn format_from_path() {
    use std::path::Path;
    assert_eq!(
        ExportFormat::from_path(Path::new("out.stl")),
        Some(ExportFormat::Stl)
    );
    assert_eq!(
        ExportFormat::from_path(Path::new("out.obj")),
        Some(ExportFormat::Obj)
    );
    assert_eq!(
        ExportFormat::from_path(Path::new("out.ply")),
        Some(ExportFormat::Ply)
    );
    assert_eq!(
        ExportFormat::from_path(Path::new("out.glb")),
        Some(ExportFormat::Glb)
    );
    assert_eq!(ExportFormat::from_path(Path::new("out.xyz")), None);
}

#[test]
fn format_extensions() {
    assert_eq!(ExportFormat::Stl.extension(), "stl");
    assert_eq!(ExportFormat::Obj.extension(), "obj");
    assert_eq!(ExportFormat::Ply.extension(), "ply");
    assert_eq!(ExportFormat::Glb.extension(), "glb");
}

// ── Empty mesh guard ──────────────────────────────────────────────────────────

#[test]
fn empty_mesh_returns_error() {
    use _core::mesher::Mesh;
    let empty = Mesh {
        vertices: vec![],
        triangles: vec![],
    };
    for fmt in [
        ExportFormat::Stl,
        ExportFormat::Obj,
        ExportFormat::Ply,
        ExportFormat::Glb,
    ] {
        assert!(
            to_bytes(&empty, fmt).is_err(),
            "{fmt:?} should error on empty mesh"
        );
    }
}

// ── STL ───────────────────────────────────────────────────────────────────────

#[test]
fn stl_minimum_size() {
    let m = sphere_mesh();
    let bytes = to_bytes(&m, ExportFormat::Stl).unwrap();
    // 80 header + 4 count + n_tri * 50
    let expected = 84 + m.triangle_count() * 50;
    assert_eq!(bytes.len(), expected);
}

#[test]
fn stl_triangle_count_matches() {
    let m = sphere_mesh();
    let bytes = to_bytes(&m, ExportFormat::Stl).unwrap();
    let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap());
    assert_eq!(count as usize, m.triangle_count());
}

#[test]
fn stl_non_zero_size() {
    let bytes = to_bytes(&sphere_mesh(), ExportFormat::Stl).unwrap();
    assert!(bytes.len() > 84, "STL must have at least one triangle");
}

#[test]
fn stl_cube_and_sphere_different_sizes() {
    let stl_sphere = to_bytes(&sphere_mesh(), ExportFormat::Stl).unwrap();
    let stl_cube = to_bytes(&cube_mesh(), ExportFormat::Stl).unwrap();
    // Both valid but different triangle counts.
    assert_ne!(stl_sphere.len(), stl_cube.len());
}

// ── OBJ ───────────────────────────────────────────────────────────────────────

#[test]
fn obj_is_valid_utf8() {
    let bytes = to_bytes(&sphere_mesh(), ExportFormat::Obj).unwrap();
    assert!(std::str::from_utf8(&bytes).is_ok());
}

#[test]
fn obj_has_vertex_lines() {
    let m = sphere_mesh();
    let bytes = to_bytes(&m, ExportFormat::Obj).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();

    let v_count = text.lines().filter(|l| l.starts_with("v ")).count();
    assert_eq!(v_count, m.vertex_count(), "vertex line count mismatch");
}

#[test]
fn obj_has_normal_lines() {
    let m = sphere_mesh();
    let bytes = to_bytes(&m, ExportFormat::Obj).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();

    let vn_count = text.lines().filter(|l| l.starts_with("vn ")).count();
    assert_eq!(vn_count, m.vertex_count(), "vertex normal count mismatch");
}

#[test]
fn obj_has_face_lines() {
    let m = sphere_mesh();
    let bytes = to_bytes(&m, ExportFormat::Obj).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();

    let f_count = text.lines().filter(|l| l.starts_with('f')).count();
    assert_eq!(f_count, m.triangle_count(), "face line count mismatch");
}

#[test]
fn obj_face_indices_are_1_indexed() {
    let bytes = to_bytes(&sphere_mesh(), ExportFormat::Obj).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    // No face should reference index 0 (OBJ is 1-based).
    for line in text.lines().filter(|l| l.starts_with('f')) {
        assert!(
            !line.contains("f 0"),
            "OBJ face index 0 found — must be 1-based"
        );
    }
}

// ── PLY ───────────────────────────────────────────────────────────────────────

#[test]
fn ply_starts_with_magic() {
    let bytes = to_bytes(&sphere_mesh(), ExportFormat::Ply).unwrap();
    assert!(bytes.starts_with(b"ply\n"));
}

/// Find the end of the PLY ASCII header in raw bytes.
fn ply_header_end(bytes: &[u8]) -> usize {
    let marker = b"end_header\n";
    bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("PLY missing end_header")
        + marker.len()
}

#[test]
fn ply_header_contains_element_counts() {
    let m = sphere_mesh();
    let bytes = to_bytes(&m, ExportFormat::Ply).unwrap();
    let end = ply_header_end(&bytes);
    // The header is pure ASCII — safe to parse as UTF-8.
    let header = std::str::from_utf8(&bytes[..end]).unwrap();

    assert!(
        header.contains(&format!("element vertex {}", m.vertex_count())),
        "PLY header missing vertex count"
    );
    assert!(
        header.contains(&format!("element face {}", m.triangle_count())),
        "PLY header missing face count"
    );
}

#[test]
fn ply_binary_size_is_correct() {
    let m = sphere_mesh();
    let bytes = to_bytes(&m, ExportFormat::Ply).unwrap();
    let end = ply_header_end(&bytes);
    let binary = &bytes[end..];

    let expected_binary = m.vertex_count() * 24 + m.triangle_count() * 13;
    assert_eq!(
        binary.len(),
        expected_binary,
        "PLY binary body size mismatch"
    );
}

// ── GLB ───────────────────────────────────────────────────────────────────────

#[test]
fn glb_magic_bytes() {
    let bytes = to_bytes(&sphere_mesh(), ExportFormat::Glb).unwrap();
    // Magic: b"glTF" = 0x46_54_6C_67 in LE bytes
    assert_eq!(&bytes[0..4], b"glTF");
}

#[test]
fn glb_version_is_2() {
    let bytes = to_bytes(&sphere_mesh(), ExportFormat::Glb).unwrap();
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    assert_eq!(version, 2);
}

#[test]
fn glb_total_length_matches_actual() {
    let bytes = to_bytes(&sphere_mesh(), ExportFormat::Glb).unwrap();
    let stated = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    assert_eq!(stated, bytes.len());
}

#[test]
fn glb_json_chunk_type() {
    let bytes = to_bytes(&sphere_mesh(), ExportFormat::Glb).unwrap();
    let chunk_type = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    assert_eq!(chunk_type, 0x4E4F_534A, "JSON chunk type mismatch");
}

#[test]
fn glb_json_chunk_is_valid_utf8_and_json() {
    let bytes = to_bytes(&sphere_mesh(), ExportFormat::Glb).unwrap();
    let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let json_slice = &bytes[20..20 + json_len];
    let json_str = std::str::from_utf8(json_slice)
        .expect("GLB JSON chunk is not valid UTF-8")
        .trim_end_matches(' ');
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).expect("GLB JSON chunk is not valid JSON");
    assert_eq!(parsed["asset"]["version"], "2.0");
}

#[test]
fn glb_bin_chunk_type() {
    let bytes = to_bytes(&sphere_mesh(), ExportFormat::Glb).unwrap();
    let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let bin_hdr = 20 + json_len; // start of BIN chunk
    let chunk_type = u32::from_le_bytes(bytes[bin_hdr + 4..bin_hdr + 8].try_into().unwrap());
    assert_eq!(chunk_type, 0x004E_4942, "BIN chunk type mismatch");
}

#[test]
fn glb_is_4_byte_aligned() {
    // Both chunks must be padded to 4-byte boundaries per the GLB spec.
    let bytes = to_bytes(&sphere_mesh(), ExportFormat::Glb).unwrap();
    let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let bin_len =
        u32::from_le_bytes(bytes[20 + json_len..24 + json_len].try_into().unwrap()) as usize;
    assert_eq!(json_len % 4, 0, "JSON chunk not 4-byte aligned");
    assert_eq!(bin_len % 4, 0, "BIN  chunk not 4-byte aligned");
}

// ── Round-trip property: all formats from cube ────────────────────────────────

#[test]
fn all_formats_produce_non_empty_output_for_cube() {
    let m = cube_mesh();
    for fmt in [
        ExportFormat::Stl,
        ExportFormat::Obj,
        ExportFormat::Ply,
        ExportFormat::Glb,
    ] {
        let bytes = to_bytes(&m, fmt).unwrap();
        assert!(!bytes.is_empty(), "{fmt:?} produced empty output");
        assert!(bytes.len() > 80, "{fmt:?} output suspiciously small");
    }
}
