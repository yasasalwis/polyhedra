"""
Phase 0 scaffold tests for the Python layer.

Tests the pure-Python constants and plane helpers — no Rust extension required.
"""

import pytest
import warnings


# ── Constants ─────────────────────────────────────────────────────────────────

def test_shape_constants_have_correct_values():
    from polyhedra.constants import CUBE, CYLINDER, SPHERE, CONE, TORUS, PYRAMID, PRISM
    assert CUBE     == 0
    assert CYLINDER == 1
    assert SPHERE   == 2
    assert CONE     == 3
    assert TORUS    == 4
    assert PYRAMID  == 5
    assert PRISM    == 6


def test_shape_constants_are_unique():
    from polyhedra.constants import CUBE, CYLINDER, SPHERE, CONE, TORUS, PYRAMID, PRISM
    values = [CUBE, CYLINDER, SPHERE, CONE, TORUS, PYRAMID, PRISM]
    assert len(values) == len(set(values))


# ── Named planes ──────────────────────────────────────────────────────────────

def test_planes_xy_name():
    from polyhedra.constants import planes
    assert planes.XY.plane_name == "XY"


def test_planes_yz_name():
    from polyhedra.constants import planes
    assert planes.YZ.plane_name == "YZ"


def test_planes_xz_name():
    from polyhedra.constants import planes
    assert planes.XZ.plane_name == "XZ"


def test_planes_origin_is_xy_alias():
    from polyhedra.constants import planes
    assert planes.Origin.plane_name == "XY"


def test_named_plane_offset_is_zero():
    from polyhedra.constants import planes
    assert planes.XY.offset_mm == 0.0


# ── Offset planes ─────────────────────────────────────────────────────────────

def test_offset_plane_stores_distance():
    from polyhedra.constants import planes
    p = planes.XY.offset(10)
    assert p.offset_mm == 10.0
    assert p.plane_name == "XY"


def test_offset_plane_negative():
    from polyhedra.constants import planes
    p = planes.XY.offset(-5)
    assert p.offset_mm == -5.0


def test_offset_plane_chained():
    from polyhedra.constants import planes
    p = planes.XY.offset(10).offset(5)
    assert p.offset_mm == 15.0


def test_offset_plane_preserves_axis():
    from polyhedra.constants import planes
    p = planes.YZ.offset(3.5)
    assert p.plane_name == "YZ"


# ── Custom planes ─────────────────────────────────────────────────────────────

def test_custom_plane_stores_point_and_normal():
    from polyhedra.constants import planes
    p = planes.custom(point=(1.0, 2.0, 3.0), normal=(0.0, 0.0, 1.0))
    assert p.point  == (1.0, 2.0, 3.0)
    assert p.normal == (0.0, 0.0, 1.0)


def test_custom_plane_name_is_custom():
    from polyhedra.constants import planes
    p = planes.custom()
    assert p.plane_name == "custom"


def test_custom_plane_default_args():
    from polyhedra.constants import planes
    p = planes.custom()
    assert p.point  == (0.0, 0.0, 0.0)
    assert p.normal == (0.0, 0.0, 1.0)


# ── Repr ──────────────────────────────────────────────────────────────────────

def test_named_plane_repr():
    from polyhedra.constants import planes
    assert "XY" in repr(planes.XY)


def test_offset_plane_repr():
    from polyhedra.constants import planes
    r = repr(planes.XY.offset(10))
    assert "XY" in r
    assert "10" in r


def test_custom_plane_repr():
    from polyhedra.constants import planes
    r = repr(planes.custom(point=(1, 2, 3), normal=(0, 0, 1)))
    assert "custom" in r


# ── Package import ────────────────────────────────────────────────────────────

def test_package_imports_constants():
    """All shape constants must be importable from the top-level package."""
    import polyhedra as ph
    assert hasattr(ph, "CUBE")
    assert hasattr(ph, "CYLINDER")
    assert hasattr(ph, "SPHERE")
    assert hasattr(ph, "CONE")
    assert hasattr(ph, "TORUS")
    assert hasattr(ph, "PYRAMID")
    assert hasattr(ph, "PRISM")


def test_package_imports_planes():
    import polyhedra as ph
    assert hasattr(ph, "planes")


def test_package_has_version():
    import polyhedra as ph
    assert hasattr(ph, "__version__")
    assert ph.__version__ == "0.1.0"


def test_core_import_warning_when_unbuilt(monkeypatch):
    """If _core is missing, an ImportWarning should be issued (not an exception)."""
    import sys
    # Remove cached module so we can re-import
    for key in list(sys.modules.keys()):
        if key.startswith("polyhedra"):
            del sys.modules[key]

    # Pretend _core does not exist
    import builtins
    real_import = builtins.__import__

    def mock_import(name, *args, **kwargs):
        if name == "polyhedra._core":
            raise ImportError("mocked: _core not found")
        return real_import(name, *args, **kwargs)

    monkeypatch.setattr(builtins, "__import__", mock_import)

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        import polyhedra  # noqa: F401 (re-import under mock)
        core_warnings = [w for w in caught if issubclass(w.category, ImportWarning)]
        assert len(core_warnings) >= 1
        assert "_core" in str(core_warnings[0].message).lower() or "maturin" in str(core_warnings[0].message).lower()
