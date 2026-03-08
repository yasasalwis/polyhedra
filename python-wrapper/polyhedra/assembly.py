"""
Assembly — combine multiple Objects with CSG operations.

An assembly is an ordered list of Objects with associated operations
(place/union, cut/difference, intersect) that reduce to a single Object
when meshed or rendered.
"""

from __future__ import annotations

from typing import Optional

from .object import Object


class Assembly:
    """
    Combine multiple ``Object`` instances into one solid.

    Each added object is tagged with a CSG operation
    (``place`` = union, ``cut`` = difference, ``intersect`` = intersection).
    Call ``to_object()`` to collapse the assembly into a single ``Object``,
    or call ``render()`` directly.

    Example::

        import polyhedra as ph

        bracket = (
            ph.Assembly("bracket")
            .place(ph.Object(ph.CUBE, 60, 40, 5))
            .place(ph.Object(ph.CUBE, 60, 5, 40).translate(0, 0, 5))
            .cut(ph.Object(ph.CYLINDER, 1.6, 20).translate(-25, -15, 0))
            .render("bracket.stl")
        )
    """

    def __init__(self, name: str = "assembly") -> None:
        self._name: str = name
        # List of (Object, op_str) where op_str in {"union", "difference", "intersection"}
        self._steps: list[tuple[Object, str]] = []

    # ── Build operations ──────────────────────────────────────────────────────

    def place(self, obj: Object) -> "Assembly":
        """Add ``obj`` to the assembly with a union (additive) operation."""
        self._steps.append((obj, "union"))
        return self

    def cut(self, obj: Object) -> "Assembly":
        """Subtract ``obj`` from the assembly."""
        self._steps.append((obj, "difference"))
        return self

    def join(self, obj: Object) -> "Assembly":
        """Alias for ``place`` — adds ``obj`` with a union operation."""
        return self.place(obj)

    def intersect(self, obj: Object) -> "Assembly":
        """Keep only the volume shared with ``obj``."""
        self._steps.append((obj, "intersection"))
        return self

    def subtract(self, obj: Object) -> "Assembly":
        """Alias for ``cut``."""
        return self.cut(obj)

    # ── Reduce ────────────────────────────────────────────────────────────────

    def to_object(self) -> Object:
        """
        Collapse the assembly into a single ``Object``.

        Returns:
            Object: the fully combined geometry.

        Raises:
            ValueError: if the assembly has no steps.
        """
        if not self._steps:
            raise ValueError(
                f"Assembly '{self._name}' is empty. "
                "Add geometry with .place(), .cut(), or .intersect()."
            )
        result, _op = self._steps[0]
        for obj, op in self._steps[1:]:
            if op == "union":
                result = result.union(obj)
            elif op == "difference":
                result = result.difference(obj)
            elif op == "intersection":
                result = result.intersection(obj)
        return result

    # ── Shortcuts that delegate to Object ─────────────────────────────────────

    def mesh(self, **kwargs):
        """Mesh the assembly — equivalent to ``assembly.to_object().mesh(...)``."""
        return self.to_object().mesh(**kwargs)

    def to_bytes(self, fmt: str = "stl", **kwargs) -> bytes:
        """
        Mesh and export to bytes.

        Args:
            fmt: ``"stl"``, ``"obj"``, ``"ply"``, or ``"glb"``.
        """
        return self.to_object().to_bytes(fmt, **kwargs)

    def render(
        self,
        path: str,
        *,
        resolution: Optional[int] = None,
        refinement: Optional[float] = None,
        bounds: Optional[float] = None,
    ) -> "Assembly":
        """
        Mesh the assembly and write it to ``path``.

        Returns ``self`` for chaining.

        Args:
            path:       Output file path (format from extension).
            resolution: Voxels per axis.
            refinement: Voxel size in mm.
            bounds:     Half-size of meshing volume in mm.
        """
        self.to_object().render(
            path,
            resolution=resolution,
            refinement=refinement,
            bounds=bounds,
        )
        return self

    # ── Inspection ────────────────────────────────────────────────────────────

    def __len__(self) -> int:
        return len(self._steps)

    def __repr__(self) -> str:
        ops = ", ".join(op for _, op in self._steps)
        return f"Assembly('{self._name}', steps=[{ops}])"
