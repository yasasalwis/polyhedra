"""
Shape type constants and construction plane helpers.

These values are passed to ph.Object() and ph.Sketch() constructors
and are read by the Rust kernel via PyO3.
"""

from __future__ import annotations

# ── Shape type constants ───────────────────────────────────────────────────
# Integer IDs that match the ShapeType enum in the Rust kernel.

CUBE     = 0  # ph.Object(ph.CUBE,     width, depth, height)
CYLINDER = 1  # ph.Object(ph.CYLINDER, radius, height)
SPHERE   = 2  # ph.Object(ph.SPHERE,   radius)
CONE     = 3  # ph.Object(ph.CONE,     base_radius, top_radius, height)
TORUS    = 4  # ph.Object(ph.TORUS,    major_radius, minor_radius)
PYRAMID  = 5  # ph.Object(ph.PYRAMID,  base_width, base_depth, height)
PRISM    = 6  # ph.Object(ph.PRISM,    sides, flat_to_flat, height)


# ── Plane helpers ─────────────────────────────────────────────────────────

class _OffsetPlane:
    """
    A standard axis-aligned plane offset along its normal by a distance.

    Created by calling .offset(distance_mm) on a _NamedPlane.
    Passed to ph.Sketch() to define the sketch plane.
    """

    def __init__(self, name: str, offset_mm: float) -> None:
        self._name      = name
        self._offset_mm = float(offset_mm)

    def offset(self, distance_mm: float) -> "_OffsetPlane":
        """Return a new plane further offset by distance_mm."""
        return _OffsetPlane(self._name, self._offset_mm + distance_mm)

    # Properties read by the Rust kernel via PyO3
    @property
    def plane_name(self) -> str:
        return self._name

    @property
    def offset_mm(self) -> float:
        return self._offset_mm

    def __repr__(self) -> str:
        return f"Plane({self._name}, offset={self._offset_mm}mm)"


class _NamedPlane:
    """
    A standard axis-aligned construction plane (XY, YZ, or XZ).

    Usage:
        sk = ph.Sketch(ph.planes.XY)          # sketch on XY plane at Z=0
        sk = ph.Sketch(ph.planes.XY.offset(10))  # sketch at Z=10mm
    """

    def __init__(self, name: str) -> None:
        self._name = name

    def offset(self, distance_mm: float) -> _OffsetPlane:
        """Create a parallel plane at distance_mm from this plane."""
        return _OffsetPlane(self._name, distance_mm)

    @property
    def plane_name(self) -> str:
        return self._name

    @property
    def offset_mm(self) -> float:
        return 0.0

    def __repr__(self) -> str:
        return f"Plane({self._name})"


class _CustomPlane:
    """
    A user-defined plane specified by a point on the plane and its normal.

    Usage:
        ph.planes.custom(point=(0, 0, 5), normal=(0, 0, 1))
    """

    def __init__(
        self,
        point:  tuple[float, float, float],
        normal: tuple[float, float, float],
    ) -> None:
        self._point  = tuple(float(v) for v in point)
        self._normal = tuple(float(v) for v in normal)

    @property
    def point(self) -> tuple[float, float, float]:
        return self._point  # type: ignore[return-value]

    @property
    def normal(self) -> tuple[float, float, float]:
        return self._normal  # type: ignore[return-value]

    @property
    def plane_name(self) -> str:
        return "custom"

    def __repr__(self) -> str:
        return f"Plane(custom, point={self._point}, normal={self._normal})"


class _PlanesNamespace:
    """
    Predefined construction planes accessible as ph.planes.<name>.

    Standard planes:
        ph.planes.XY      — Z-normal, at Z = 0  (same as ph.planes.Origin)
        ph.planes.YZ      — X-normal, at X = 0
        ph.planes.XZ      — Y-normal, at Y = 0
        ph.planes.Origin  — alias for XY

    Offset planes:
        ph.planes.XY.offset(10)    — XY plane at Z = +10 mm
        ph.planes.XY.offset(-5)    — XY plane at Z = -5 mm

    Custom planes:
        ph.planes.custom(point=(0,0,5), normal=(0,0,1))
    """

    XY:     _NamedPlane = _NamedPlane("XY")
    YZ:     _NamedPlane = _NamedPlane("YZ")
    XZ:     _NamedPlane = _NamedPlane("XZ")
    Origin: _NamedPlane = _NamedPlane("XY")   # alias for XY at Z = 0

    @staticmethod
    def custom(
        point:  tuple[float, float, float] = (0.0, 0.0, 0.0),
        normal: tuple[float, float, float] = (0.0, 0.0, 1.0),
    ) -> _CustomPlane:
        """Define a plane by a point on the plane and its outward normal."""
        return _CustomPlane(point, normal)


# Singleton — import as `from polyhedra import planes` or use `ph.planes.*`
planes = _PlanesNamespace()
