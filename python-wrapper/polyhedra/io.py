"""
File I/O helpers — load and compile ``.polyh`` files.

``load(path)``     → parse a ``.polyh`` file into an ``Object`` (single define block).
``compile_polyh()`` → parse, assemble, and export as defined in ``main.polyh``.
``validate(path)``  → check syntax without meshing; return list of errors.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Optional, Union

from .object import Object, _build_primitive, _core
from .assembly import Assembly


# ── Public API ─────────────────────────────────────────────────────────────────

def load(path: Union[str, Path]) -> Object:
    """
    Parse a ``.polyh`` file and return the first ``define`` block as an ``Object``.

    The file must contain exactly one ``define`` block with a single primitive
    (full multi-primitive and assembly support is available via ``compile_polyh``).

    Args:
        path: Path to the ``.polyh`` file.

    Returns:
        Object: the defined geometry.

    Raises:
        FileNotFoundError: if ``path`` does not exist.
        ValueError:        if the file contains no ``define`` blocks or the
                           primitive cannot be evaluated.
        RuntimeError:      if ``_core`` is not compiled.

    Example::

        bracket = ph.load("bracket.polyh")
        bracket.render("bracket.stl")
    """
    path = Path(path)
    if not path.exists():
        raise FileNotFoundError(f"polyhedra.load: file not found: {path}")

    from ._polyh_eval import eval_file
    return eval_file(path)


def compile_polyh(
    path: Union[str, Path],
    *,
    output: str = "stl",
    resolution: Optional[int] = None,
    refinement: Optional[float] = None,
    bounds: Optional[float] = None,
) -> bytes:
    """
    Parse, assemble, and export a ``.polyh`` file.

    If the file contains an ``export`` block, its ``format`` and ``file``
    settings are used (but ``output``, ``resolution``, and ``refinement``
    arguments override them if provided).

    Args:
        path:       Path to the ``.polyh`` file (usually ``main.polyh``).
        output:     Export format: ``"stl"``, ``"obj"``, ``"ply"``, ``"glb"``.
        resolution: Voxels per axis.
        refinement: Voxel size in mm.
        bounds:     Half-size of meshing volume in mm.

    Returns:
        bytes: the exported mesh file content.

    Raises:
        FileNotFoundError: if ``path`` does not exist.
        ValueError:        if the file has no ``assemble`` block.
        RuntimeError:      if ``_core`` is not compiled.
    """
    path = Path(path)
    if not path.exists():
        raise FileNotFoundError(f"polyhedra.compile: file not found: {path}")

    from ._polyh_eval import eval_file_assembly
    obj = eval_file_assembly(path)
    return obj.to_bytes(output, resolution=resolution, refinement=refinement, bounds=bounds)


def validate(path: Union[str, Path]) -> list[str]:
    """
    Check ``.polyh`` syntax and return a list of error messages.

    Returns an empty list if the file is valid.

    Args:
        path: Path to the ``.polyh`` file.

    Returns:
        list[str]: error messages (empty = no errors).

    Example::

        errors = ph.validate("bracket.polyh")
        if errors:
            for e in errors:
                print(e)
    """
    path = Path(path)
    if not path.exists():
        return [f"File not found: {path}"]

    try:
        from ._polyh_eval import eval_file
        eval_file(path)
        return []
    except Exception as exc:
        return [str(exc)]
