"""
Object — the primary user-facing geometry type.

Every solid in polyhedra is an ``Object``.  It wraps a Rust SDF node
and exposes a chainable Python API for CSG operations, transformations,
manipulations, and export.
"""

from __future__ import annotations

from typing import Optional


# ── Internal helper ────────────────────────────────────────────────────────────

def _core():
    """Lazily import the compiled Rust extension; raise a helpful error if missing."""
    try:
        from polyhedra import _core as _c
        return _c
    except ImportError:
        raise RuntimeError(
            "polyhedra._core (Rust extension) is not compiled.\n"
            "Build it with:  cd python/  &&  maturin develop"
        ) from None


# ── Object ─────────────────────────────────────────────────────────────────────

class Object:
    """
    A 3-D solid geometry object backed by a Rust SDF node.

    Create from a primitive type constant and dimensions::

        import polyhedra as ph

        cube  = ph.Object(ph.CUBE,     60,  40, 20)   # width, depth, height
        cyl   = ph.Object(ph.CYLINDER,  5,  30)        # radius, height
        ball  = ph.Object(ph.SPHERE,    7.5)           # radius
        donut = ph.Object(ph.TORUS,    20,  4)         # major, minor
        cone  = ph.Object(ph.CONE,      8,  0, 20)    # base_r, top_r, height
        pyr   = ph.Object(ph.PYRAMID,  10, 10, 15)    # bw, bd, height
        hex_  = ph.Object(ph.PRISM,     6, 12, 20)    # sides, flat-to-flat, height

    All operation methods return a new ``Object`` — the original is unchanged::

        part = (ph.Object(ph.CUBE, 60, 40, 20)
                  .shell(2.5)
                  .translate(0, 0, 10)
                  .render("housing.stl"))
    """

    __slots__ = ("_node", "_bounds")

    # ── Construction ──────────────────────────────────────────────────────────

    def __init__(self, shape: int, *args: float) -> None:
        """
        Create a primitive Object.

        Args:
            shape: One of ``ph.CUBE``, ``ph.SPHERE``, ``ph.CYLINDER``,
                   ``ph.CONE``, ``ph.TORUS``, ``ph.PYRAMID``, ``ph.PRISM``.
            *args: Dimensions — see the class docstring for each shape.
        """
        self._node, self._bounds = _build_primitive(shape, args)

    @classmethod
    def _from_node(cls, node, bounds: float) -> "Object":
        """Internal constructor — wrap an existing SdfNode."""
        obj = object.__new__(cls)
        obj._node   = node
        obj._bounds = float(bounds)
        return obj

    def _wrap(self, node, extra: float = 0.0) -> "Object":
        """Return a new Object sharing the same bounds ± extra."""
        return Object._from_node(node, self._bounds + extra)

    # ── Boolean CSG ───────────────────────────────────────────────────────────

    def union(self, other: "Object") -> "Object":
        """Combine self and ``other`` (CSG union)."""
        return Object._from_node(
            self._node.union(other._node),
            max(self._bounds, other._bounds),
        )

    def difference(self, other: "Object") -> "Object":
        """Subtract ``other`` from self (CSG difference / cut)."""
        return Object._from_node(
            self._node.difference(other._node),
            self._bounds,
        )

    def intersection(self, other: "Object") -> "Object":
        """Keep only the volume shared by self and ``other``."""
        return Object._from_node(
            self._node.intersection(other._node),
            min(self._bounds, other._bounds),
        )

    def smooth_union(self, other: "Object", blend: float = 5.0) -> "Object":
        """
        Union with a smooth rounded fillet at the join.

        Args:
            other: The second geometry.
            blend: Fillet radius in mm.  Higher = softer join.
        """
        return Object._from_node(
            self._node.smooth_union(other._node, float(blend)),
            max(self._bounds, other._bounds) + blend,
        )

    def smooth_difference(self, other: "Object", blend: float = 5.0) -> "Object":
        """Difference with a smooth chamfer at the cut edge."""
        return Object._from_node(
            self._node.smooth_difference(other._node, float(blend)),
            self._bounds,
        )

    def chamfer_union(self, other: "Object", radius: float = 2.0) -> "Object":
        """Union with a flat 45-degree chamfer at the join."""
        return Object._from_node(
            self._node.chamfer_union(other._node, float(radius)),
            max(self._bounds, other._bounds),
        )

    # ── Manipulations ─────────────────────────────────────────────────────────

    def fillet(self, radius: float) -> "Object":
        """
        Round all edges and corners by ``radius`` mm.

        Implemented as an exact SDF offset — produces smooth fillets
        with no mesh post-processing.

        Args:
            radius: Fillet radius in mm (must be positive).
        """
        return self._wrap(self._node.offset(float(radius)), extra=float(radius))

    def chamfer(self, radius: float, _angle: float = 45.0) -> "Object":
        """
        Bevel all edges by ``radius`` mm.

        Currently uses an SDF offset (same as ``fillet``).  The ``_angle``
        parameter is accepted for API compatibility and reserved for a future
        flat-chamfer implementation.

        Args:
            radius: Chamfer size in mm.
            _angle: Reserved (default 45°).
        """
        return self._wrap(self._node.offset(float(radius)), extra=float(radius))

    def shell(self, thickness: float) -> "Object":
        """
        Hollow out the object, leaving a wall of ``thickness`` mm.

        Args:
            thickness: Wall thickness in mm.
        """
        return self._wrap(self._node.shell(float(thickness)))

    def offset(self, amount: float) -> "Object":
        """
        Grow (positive) or shrink (negative) the object by ``amount`` mm.

        Positive offsets round convex edges; negative offsets trim them
        and can produce interior cavities if large enough.

        Args:
            amount: Offset distance in mm.
        """
        return self._wrap(self._node.offset(float(amount)), extra=max(0.0, amount))

    def elongate(
        self,
        x: float = 0.0,
        y: float = 0.0,
        z: float = 0.0,
    ) -> "Object":
        """
        Stretch the object by extruding its interior along each axis.

        Args:
            x, y, z: Extension amounts in mm per axis.
        """
        extra = max(float(x), float(y), float(z))
        return self._wrap(self._node.elongate(float(x), float(y), float(z)), extra=extra)

    # ── Transforms ────────────────────────────────────────────────────────────

    def translate(
        self,
        x: float = 0.0,
        y: float = 0.0,
        z: float = 0.0,
    ) -> "Object":
        """
        Move the object by ``(x, y, z)`` mm.

        Args:
            x, y, z: Translation in mm per axis.
        """
        mag = (float(x) ** 2 + float(y) ** 2 + float(z) ** 2) ** 0.5
        return self._wrap(self._node.translate(float(x), float(y), float(z)), extra=mag)

    def rotate_x(self, degrees: float) -> "Object":
        """Rotate about the X axis by ``degrees``."""
        return self._wrap(self._node.rotate_x(float(degrees)))

    def rotate_y(self, degrees: float) -> "Object":
        """Rotate about the Y axis by ``degrees``."""
        return self._wrap(self._node.rotate_y(float(degrees)))

    def rotate_z(self, degrees: float) -> "Object":
        """Rotate about the Z axis by ``degrees``."""
        return self._wrap(self._node.rotate_z(float(degrees)))

    def scale(self, factor: float) -> "Object":
        """Scale uniformly by ``factor``."""
        return Object._from_node(
            self._node.scale(float(factor)),
            self._bounds * float(factor),
        )

    def scale_xyz(self, sx: float, sy: float, sz: float) -> "Object":
        """Scale independently along X, Y, Z."""
        return Object._from_node(
            self._node.scale_xyz(float(sx), float(sy), float(sz)),
            self._bounds * max(float(sx), float(sy), float(sz)),
        )

    def mirror_x(self) -> "Object":
        """Mirror across the YZ plane (reflect X)."""
        return self._wrap(self._node.mirror_x())

    def mirror_y(self) -> "Object":
        """Mirror across the XZ plane (reflect Y)."""
        return self._wrap(self._node.mirror_y())

    def mirror_z(self) -> "Object":
        """Mirror across the XY plane (reflect Z)."""
        return self._wrap(self._node.mirror_z())

    # ── Meshing & export ──────────────────────────────────────────────────────

    def mesh(
        self,
        *,
        resolution: Optional[int] = None,
        refinement: Optional[float] = None,
        bounds: Optional[float] = None,
    ):
        """
        Triangulate the object using Dual Contouring.

        Args:
            resolution: Voxels per axis (default 32).  Higher = finer mesh.
            refinement: Voxel size in mm (alternative to ``resolution``).
                        The number of voxels is computed as
                        ``int(bounds * 2 / refinement)``, clamped to [8, 512].
            bounds:     Half-size of the meshing volume in mm.
                        Defaults to ``auto`` (1.1× the object's estimated extent).

        Returns:
            _core.Mesh: the triangulated mesh.

        Raises:
            RuntimeError: if ``_core`` is not compiled.
            ValueError:   if the resulting mesh is empty (geometry outside bounds).
        """
        c   = _core()
        res = _resolve_resolution(resolution, refinement, self._bounds)
        b   = float(bounds) if bounds is not None else self._bounds * 1.1
        return c.mesh_sdf(self._node, res, b)

    def to_bytes(
        self,
        fmt: str = "stl",
        *,
        resolution: Optional[int] = None,
        refinement: Optional[float] = None,
        bounds: Optional[float] = None,
    ) -> bytes:
        """
        Mesh the object and return the file contents as ``bytes``.

        Args:
            fmt:        Export format: ``"stl"``, ``"obj"``, ``"ply"``, or ``"glb"``.
            resolution: Voxels per axis.
            refinement: Voxel size in mm.
            bounds:     Half-size of meshing volume in mm.

        Returns:
            bytes: serialised mesh file.
        """
        m      = self.mesh(resolution=resolution, refinement=refinement, bounds=bounds)
        export = getattr(m, f"to_{fmt.lower().strip('.')}")
        return bytes(export())

    def render(
        self,
        path: str,
        *,
        resolution: Optional[int] = None,
        refinement: Optional[float] = None,
        bounds: Optional[float] = None,
    ) -> "Object":
        """
        Mesh the object and write it to ``path``.

        The export format is inferred from the file extension
        (``.stl``, ``.obj``, ``.ply``, ``.glb``, ``.gltf``).

        Returns ``self`` so you can chain further operations or renders::

            (ph.Object(ph.SPHERE, 10)
               .fillet(1)
               .render("sphere_hi.stl", resolution=64)
               .render("sphere_lo.stl", resolution=16))

        Args:
            path:       Output file path (format inferred from extension).
            resolution: Voxels per axis.
            refinement: Voxel size in mm.
            bounds:     Half-size of meshing volume in mm.

        Returns:
            self
        """
        m = self.mesh(resolution=resolution, refinement=refinement, bounds=bounds)
        m.save(path)
        return self

    # ── Evaluation ────────────────────────────────────────────────────────────

    def distance(self, x: float, y: float, z: float) -> float:
        """
        Evaluate the SDF at world-space point ``(x, y, z)``.

        Returns:
            float: negative inside, 0 on the surface, positive outside.
        """
        return float(self._node.distance(float(x), float(y), float(z)))

    # ── Python operator overloads ─────────────────────────────────────────────

    def __add__(self, other: "Object") -> "Object":
        """``a + b`` → union."""
        return self.union(other)

    def __sub__(self, other: "Object") -> "Object":
        """``a - b`` → difference."""
        return self.difference(other)

    def __and__(self, other: "Object") -> "Object":
        """``a & b`` → intersection."""
        return self.intersection(other)

    def __repr__(self) -> str:
        return f"Object(bounds≈{self._bounds:.1f}mm)"


# ── Helpers ────────────────────────────────────────────────────────────────────

def _build_primitive(shape: int, args: tuple) -> tuple:
    """
    Build a _core.SdfNode for the given shape type and arguments.

    Returns:
        (SdfNode, bounds_hint_mm)
    """
    from .constants import CUBE, CYLINDER, SPHERE, CONE, TORUS, PYRAMID, PRISM
    c = _core()

    if shape == CUBE:
        w, d, h = float(args[0]), float(args[1]), float(args[2])
        return c.cube(w, d, h), max(w, d, h) * 0.56
    if shape == SPHERE:
        r = float(args[0])
        return c.sphere(r), r * 1.1
    if shape == CYLINDER:
        r, h = float(args[0]), float(args[1])
        return c.cylinder(r, h), max(r * 2, h) * 0.56
    if shape == CONE:
        br, tr, h = float(args[0]), float(args[1]), float(args[2])
        return c.cone(br, tr, h), max(br * 2, tr * 2, h) * 0.56
    if shape == TORUS:
        major, minor = float(args[0]), float(args[1])
        return c.torus(major, minor), (major + minor) * 1.1
    if shape == PYRAMID:
        bw, bd, h = float(args[0]), float(args[1]), float(args[2])
        return c.pyramid(bw, bd, h), max(bw, bd, h) * 0.6
    if shape == PRISM:
        sides = int(args[0])
        ftf   = float(args[1])
        h     = float(args[2])
        return c.prism(sides, ftf, h), max(ftf, h) * 0.56
    raise ValueError(
        f"Unknown shape type {shape!r}. "
        f"Use ph.CUBE, ph.SPHERE, ph.CYLINDER, ph.CONE, "
        f"ph.TORUS, ph.PYRAMID, or ph.PRISM."
    )


def _resolve_resolution(
    resolution: Optional[int],
    refinement: Optional[float],
    bounds: float,
) -> int:
    """
    Convert user-facing resolution/refinement args to a voxel count.

    Priority: resolution > refinement > default (32).
    """
    if resolution is not None:
        return max(4, int(resolution))
    if refinement is not None:
        diameter = bounds * 2.0
        voxels   = int(diameter / float(refinement))
        return max(8, min(512, voxels))
    return 32
