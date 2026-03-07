"""
polyhedra — Plain-English 3D design library.

Rust-powered geometry kernel exposed through a clean Python API.

Quick start:
    import polyhedra as ph

    # Create and manipulate an object
    cube = ph.Object(ph.CUBE, 10, 10, 10)
    cube.chamfer(1.4, 15)
    cube.render('cube.stl', refinement=0.001)

    # Chainable API
    part = ph.Object(ph.CYLINDER, 5, 20).fillet(2.0).render('cyl.stl', refinement=0.01)

    # Load an existing .polyh file
    bracket = ph.load('bracket.polyh').chamfer(1.5)
    bracket.render('bracket.stl', refinement=0.001)

    # Compile a main.polyh
    result = ph.compile('main.polyh', output='stl', refinement=0.001)
"""

from __future__ import annotations

# ── Shape type constants and plane helpers (pure Python) ──────────────────
from .constants import (
    CUBE,
    CYLINDER,
    SPHERE,
    CONE,
    TORUS,
    PYRAMID,
    PRISM,
    planes,
)

__version__ = "0.1.0"

# ── Rust extension (_core) ────────────────────────────────────────────────
# Imported at module load time. If _core is not yet compiled, a clear
# warning guides the user to `maturin develop`.
try:
    from ._core import (  # noqa: F401  (re-exported for public use)
        Object,
        Sketch,
        Assembly,
        load,
        compile as compile_polyh,
        validate,
        watch,
        join,
        cut,
        intersect,
        boundary_fill,
        loft,
        sweep,
        ruled,
        stitch,
        plane_at_angle,
        plane_mid,
        plane_tangent,
        plane_along_path,
        axis_of,
        axis,
        axis_along,
        point_at,
    )
    _core_loaded = True
except ImportError:
    _core_loaded = False
    import warnings

    warnings.warn(
        "\n\n"
        "  polyhedra: Rust extension (_core) not found.\n"
        "  Build it with:\n\n"
        "      cd python/\n"
        "      maturin develop\n\n"
        "  See docs/quick-start/installation.md for full instructions.\n",
        ImportWarning,
        stacklevel=2,
    )


def _require_core() -> None:
    """Raise a clear error if the Rust extension is not compiled."""
    if not _core_loaded:
        raise RuntimeError(
            "polyhedra._core (Rust extension) is not compiled.\n"
            "Run `maturin develop` in the python/ directory."
        )


__all__ = [
    # Constants
    "CUBE", "CYLINDER", "SPHERE", "CONE", "TORUS", "PYRAMID", "PRISM",
    "planes",
    # Core types (from Rust)
    "Object", "Sketch", "Assembly",
    # Functions
    "load", "compile_polyh", "validate", "watch",
    "join", "cut", "intersect", "boundary_fill", "loft", "sweep",
    "ruled", "stitch",
    # Construction geometry
    "plane_at_angle", "plane_mid", "plane_tangent", "plane_along_path",
    "axis_of", "axis", "axis_along", "point_at",
    # Meta
    "__version__",
]
