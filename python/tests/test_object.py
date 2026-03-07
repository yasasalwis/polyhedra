"""
Tests for the Object class.

Tests are grouped into two sections:
  1. Pure-Python checks — run without _core (constants, repr, type hierarchy).
  2. _core integration — skipped when _core is not compiled.
"""

from __future__ import annotations

import pytest
import warnings

# Detect whether _core is available.
try:
    import polyhedra._core  # noqa: F401
    CORE_AVAILABLE = True
except ImportError:
    CORE_AVAILABLE = False

skip_no_core = pytest.mark.skipif(
    not CORE_AVAILABLE,
    reason="_core extension not compiled (run maturin develop)",
)


# ── Import smoke ───────────────────────────────────────────────────────────────

def test_object_class_importable():
    from polyhedra.object import Object
    assert callable(Object)


def test_assembly_class_importable():
    from polyhedra.assembly import Assembly
    assert callable(Assembly)


def test_package_exposes_object():
    import polyhedra as ph
    assert hasattr(ph, "Object")
    assert hasattr(ph, "Assembly")


# ── Assembly (pure-Python, no _core) ──────────────────────────────────────────

def test_assembly_empty_raises():
    from polyhedra.assembly import Assembly
    asm = Assembly("empty")
    with pytest.raises(ValueError, match="empty"):
        asm.to_object()


def test_assembly_repr():
    from polyhedra.assembly import Assembly
    asm = Assembly("test")
    r = repr(asm)
    assert "test" in r


def test_assembly_len_zero():
    from polyhedra.assembly import Assembly
    assert len(Assembly()) == 0


# ── _core integration tests ────────────────────────────────────────────────────

@skip_no_core
def test_cube_construction():
    import polyhedra as ph
    c = ph.Object(ph.CUBE, 10, 10, 10)
    assert isinstance(c, ph.Object)


@skip_no_core
def test_sphere_construction():
    import polyhedra as ph
    s = ph.Object(ph.SPHERE, 5)
    assert isinstance(s, ph.Object)


@skip_no_core
def test_cylinder_construction():
    import polyhedra as ph
    c = ph.Object(ph.CYLINDER, 4, 20)
    assert isinstance(c, ph.Object)


@skip_no_core
def test_cone_construction():
    import polyhedra as ph
    c = ph.Object(ph.CONE, 6, 0, 15)
    assert isinstance(c, ph.Object)


@skip_no_core
def test_torus_construction():
    import polyhedra as ph
    t = ph.Object(ph.TORUS, 10, 2)
    assert isinstance(t, ph.Object)


@skip_no_core
def test_pyramid_construction():
    import polyhedra as ph
    p = ph.Object(ph.PYRAMID, 10, 10, 15)
    assert isinstance(p, ph.Object)


@skip_no_core
def test_prism_construction():
    import polyhedra as ph
    p = ph.Object(ph.PRISM, 6, 12, 20)
    assert isinstance(p, ph.Object)


@skip_no_core
def test_unknown_shape_raises():
    import polyhedra as ph
    with pytest.raises(ValueError):
        ph.Object(99, 1, 2, 3)


@skip_no_core
def test_distance_inside_cube():
    import polyhedra as ph
    c = ph.Object(ph.CUBE, 10, 10, 10)
    assert c.distance(0, 0, 0) < 0.0


@skip_no_core
def test_distance_outside_sphere():
    import polyhedra as ph
    s = ph.Object(ph.SPHERE, 5)
    assert s.distance(20, 0, 0) > 0.0


@skip_no_core
def test_union_operator():
    import polyhedra as ph
    a = ph.Object(ph.CUBE, 10, 10, 10)
    b = ph.Object(ph.SPHERE, 5)
    u = a + b
    assert isinstance(u, ph.Object)
    assert u.distance(0, 0, 0) < 0.0


@skip_no_core
def test_difference_operator():
    import polyhedra as ph
    a = ph.Object(ph.CUBE, 10, 10, 10)
    b = ph.Object(ph.SPHERE, 5)
    d = a - b
    assert d.distance(0, 0, 0) > 0.0  # centre removed by sphere


@skip_no_core
def test_intersection_operator():
    import polyhedra as ph
    a = ph.Object(ph.CUBE, 10, 10, 10)
    b = ph.Object(ph.SPHERE, 5)
    i = a & b
    assert i.distance(0, 0, 0) < 0.0  # both cover origin


@skip_no_core
def test_translate_moves_geometry():
    import polyhedra as ph
    s = ph.Object(ph.SPHERE, 5)
    t = s.translate(20, 0, 0)
    assert t.distance(0, 0, 0) > 0.0
    assert t.distance(20, 0, 0) < 0.0


@skip_no_core
def test_scale_enlarges():
    import polyhedra as ph
    s = ph.Object(ph.SPHERE, 5)       # radius 5
    big = s.scale(3)                   # radius ≈ 15
    assert big.distance(12, 0, 0) < 0.0


@skip_no_core
def test_fillet_rounds_edges():
    import polyhedra as ph
    c = ph.Object(ph.CUBE, 10, 10, 10)
    f = c.fillet(1.0)
    assert isinstance(f, ph.Object)
    # Interior is still solid
    assert f.distance(0, 0, 0) < 0.0


@skip_no_core
def test_shell_hollows():
    import polyhedra as ph
    c = ph.Object(ph.CUBE, 10, 10, 10)
    s = c.shell(1.0)
    # Deep centre should be outside (hollow)
    assert s.distance(0, 0, 0) > 0.0


@skip_no_core
def test_mirror_x_reflects():
    import polyhedra as ph
    s = ph.Object(ph.SPHERE, 3).translate(8, 0, 0)
    m = s.mirror_x()
    assert m.distance(-8, 0, 0) < 0.0
    assert m.distance( 8, 0, 0) < 0.0


@skip_no_core
def test_rotate_z_changes_shape():
    import polyhedra as ph
    c = ph.Object(ph.CUBE, 10, 2, 10)  # thin along Y
    r = c.rotate_z(90)
    # After 90° rotation the thin dimension is now along X
    assert r.distance(0.8, 0, 0) < 0.0
    assert r.distance(3.0, 0, 0) > 0.0


@skip_no_core
def test_smooth_union_blends():
    import polyhedra as ph
    a = ph.Object(ph.SPHERE, 5)
    b = ph.Object(ph.SPHERE, 5).translate(6, 0, 0)
    u = a.smooth_union(b, blend=2.0)
    # Midpoint should be inside due to blending
    assert u.distance(3, 0, 0) < 0.0


@skip_no_core
def test_chaining_returns_object():
    import polyhedra as ph
    result = (
        ph.Object(ph.CUBE, 10, 10, 10)
        .fillet(0.5)
        .shell(1.0)
        .translate(0, 0, 5)
        .scale(1.2)
    )
    assert isinstance(result, ph.Object)


@skip_no_core
def test_repr_contains_bounds():
    import polyhedra as ph
    c = ph.Object(ph.CUBE, 10, 10, 10)
    r = repr(c)
    assert "Object" in r
    assert "mm" in r


@skip_no_core
def test_to_bytes_stl():
    import polyhedra as ph
    c = ph.Object(ph.CUBE, 10, 10, 10)
    data = c.to_bytes("stl", resolution=8)
    assert len(data) > 84
    # STL triangle count at bytes 80-84
    count = int.from_bytes(data[80:84], "little")
    assert count > 0


@skip_no_core
def test_to_bytes_obj():
    import polyhedra as ph
    s = ph.Object(ph.SPHERE, 7)
    data = c = ph.Object(ph.SPHERE, 7).to_bytes("obj", resolution=10)
    text = data.decode("utf-8")
    assert "v " in text
    assert "\nf " in text


@skip_no_core
def test_to_bytes_ply():
    import polyhedra as ph
    data = ph.Object(ph.SPHERE, 7).to_bytes("ply", resolution=10)
    assert data.startswith(b"ply\n")


@skip_no_core
def test_to_bytes_glb():
    import polyhedra as ph
    data = ph.Object(ph.SPHERE, 7).to_bytes("glb", resolution=10)
    assert data[:4] == b"glTF"


# ── Assembly integration ───────────────────────────────────────────────────────

@skip_no_core
def test_assembly_place_and_cut():
    import polyhedra as ph
    asm = (
        ph.Assembly("test")
        .place(ph.Object(ph.CUBE, 10, 10, 10))
        .cut(ph.Object(ph.SPHERE, 4))
    )
    obj = asm.to_object()
    assert isinstance(obj, ph.Object)
    # Centre should be hollow (sphere removed)
    assert obj.distance(0, 0, 0) > 0.0


@skip_no_core
def test_assembly_two_place():
    import polyhedra as ph
    asm = (
        ph.Assembly()
        .place(ph.Object(ph.SPHERE, 5))
        .place(ph.Object(ph.SPHERE, 5).translate(8, 0, 0))
    )
    obj = asm.to_object()
    assert obj.distance(0, 0, 0) < 0.0
    assert obj.distance(8, 0, 0) < 0.0


@skip_no_core
def test_assembly_len():
    import polyhedra as ph
    asm = (
        ph.Assembly()
        .place(ph.Object(ph.CUBE, 10, 10, 10))
        .cut(ph.Object(ph.SPHERE, 4))
    )
    assert len(asm) == 2


# ── Resolution helper ──────────────────────────────────────────────────────────

def test_resolve_resolution_default():
    from polyhedra.object import _resolve_resolution
    assert _resolve_resolution(None, None, 50.0) == 32


def test_resolve_resolution_explicit():
    from polyhedra.object import _resolve_resolution
    assert _resolve_resolution(64, None, 50.0) == 64


def test_resolve_resolution_from_refinement():
    from polyhedra.object import _resolve_resolution
    # bounds=50mm, refinement=2mm → 100mm / 2mm = 50 voxels
    assert _resolve_resolution(None, 2.0, 50.0) == 50


def test_resolve_resolution_clamped_min():
    from polyhedra.object import _resolve_resolution
    # refinement so coarse → 8 minimum
    assert _resolve_resolution(None, 1000.0, 10.0) == 8


def test_resolve_resolution_clamped_max():
    from polyhedra.object import _resolve_resolution
    assert _resolve_resolution(None, 0.0001, 100.0) == 512
