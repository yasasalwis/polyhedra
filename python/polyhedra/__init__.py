"""
polyhedra — Plain-English 3D design library.

Rust-powered geometry kernel (Dual Contouring SDF mesher) exposed through
a clean, chainable Python API.

Quick start::

    import polyhedra as ph

    # Primitives
    cube = ph.Object(ph.CUBE, 60, 40, 20)       # width, depth, height
    ball = ph.Object(ph.SPHERE, 10)              # radius
    cyl  = ph.Object(ph.CYLINDER, 5, 30)         # radius, height

    # Chainable operations
    part = (ph.Object(ph.CUBE, 60, 40, 20)
              .shell(2.5)
              .fillet(1.0)
              .translate(0, 0, 10)
              .render("housing.stl"))

    # CSG assembly
    housing = (ph.Assembly("housing")
                  .place(ph.Object(ph.CUBE, 80, 50, 30))
                  .cut(ph.Object(ph.CYLINDER, 5, 40).translate(0, 0, 0))
                  .render("housing.stl"))

    # Load a .polyh file
    bracket = ph.load("bracket.polyh")
    bracket.fillet(1.5).render("bracket_filleted.stl")

    # Operator shortcuts
    combined = cube + ball            # union
    cut_part = cube - ball            # difference
    overlap  = cube & ball            # intersection
"""

from __future__ import annotations

# ── Pure-Python types (no Rust required) ──────────────────────────────────────

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

from .assembly import Assembly

# ── Object class and I/O (require _core at call time, not at import time) ─────

from .object import Object
from .io import load, compile_polyh, validate

__version__ = "0.1.0"

# ── Optional Rust extension ────────────────────────────────────────────────────
# Imported lazily by Object / io when first needed.
# If _core is not compiled, a clear error is raised only when geometry
# operations are actually attempted — not on `import polyhedra`.

try:
    from . import _core  # noqa: F401 — verify it's importable
    _core_available = True
except ImportError:
    _core_available = False
    import warnings
    warnings.warn(
        "\n\n"
        "  polyhedra: Rust extension (_core) not found.\n"
        "  Build it with:\n\n"
        "      cd python/\n"
        "      maturin develop\n\n"
        "  See README for full installation instructions.\n",
        ImportWarning,
        stacklevel=2,
    )

# ── Public API ─────────────────────────────────────────────────────────────────

__all__ = [
    # Shape type constants
    "CUBE", "CYLINDER", "SPHERE", "CONE", "TORUS", "PYRAMID", "PRISM",
    # Construction planes
    "planes",
    # Core classes
    "Object",
    "Assembly",
    # File I/O
    "load",
    "compile_polyh",
    "validate",
    # Meta
    "__version__",
]
