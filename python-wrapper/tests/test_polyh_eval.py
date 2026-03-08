"""
Tests for the .polyh mini-parser and AST evaluator.

The mini-parser is pure Python — these tests run without _core.
The evaluator tests require _core and are skipped when it's absent.
"""

from __future__ import annotations

import pytest

try:
    import polyhedra._core  # noqa: F401
    CORE_AVAILABLE = True
except ImportError:
    CORE_AVAILABLE = False

skip_no_core = pytest.mark.skipif(
    not CORE_AVAILABLE,
    reason="_core extension not compiled",
)


# ── Mini-parser unit tests (pure Python) ──────────────────────────────────────

from polyhedra._polyh_eval import _py_parse, _tokenise


def test_tokenise_strips_comments():
    tokens = _tokenise("define box  # this is a comment\n  end")
    assert "#" not in tokens
    assert "this" not in tokens
    assert "define" in tokens


def test_tokenise_handles_parens():
    tokens = _tokenise("place base at (1, 2, 3)")
    assert "(" in tokens
    assert "1" in tokens
    assert "3" in tokens


def test_parse_empty_source():
    ast = _py_parse("")
    assert ast == []


def test_parse_comment_only():
    ast = _py_parse("# just a comment\n# another\n")
    assert ast == []


def test_parse_use_statement():
    ast = _py_parse("use hello")
    assert len(ast) == 1
    assert ast[0] == {"kind": "use", "name": "hello"}


def test_parse_simple_define():
    src = "define box\n  cube\n    width 10\n    depth 5\n    height 3\n  end\nend"
    ast = _py_parse(src)
    assert len(ast) == 1
    d = ast[0]
    assert d["kind"]  == "define"
    assert d["name"]  == "box"
    assert len(d["primitives"]) == 1
    p = d["primitives"][0]
    assert p["kind"]  == "cube"
    assert p["props"]["width"]  == 10.0
    assert p["props"]["depth"]  == 5.0
    assert p["props"]["height"] == 3.0


def test_parse_define_with_units():
    src = "define ball\n  units mm\n  sphere\n    radius 7\n  end\nend"
    ast = _py_parse(src)
    d = ast[0]
    assert d["units"] == "mm"
    assert d["primitives"][0]["kind"] == "sphere"


def test_parse_define_with_chamfer():
    src = "define c\n  cube\n    width 10\n  end\n  chamfer 1.5\nend"
    ast = _py_parse(src)
    d = ast[0]
    assert any(m["op"] == "chamfer" and m["value"] == 1.5 for m in d["manips"])


def test_parse_define_with_fillet():
    src = "define c\n  sphere\n    radius 5\n  end\n  fillet 2\nend"
    ast = _py_parse(src)
    assert any(m["op"] == "fillet" for m in ast[0]["manips"])


def test_parse_assemble_place_origin():
    src = "assemble a\n  place box at origin\nend"
    ast = _py_parse(src)
    assert ast[0]["kind"] == "assemble"
    op = ast[0]["ops"][0]
    assert op["op"] == "place"
    assert op["name"] == "box"
    assert op["at"] == (0.0, 0.0, 0.0)


def test_parse_assemble_cut_coords():
    src = "assemble a\n  place b at origin\n  cut hole at (-25, -15, 0)\nend"
    ast = _py_parse(src)
    ops = ast[0]["ops"]
    assert ops[1]["op"] == "cut"
    assert ops[1]["at"] == (-25.0, -15.0, 0.0)


def test_parse_export_block():
    src = "export\n  format stl\n  quality high\n  file out.stl\nend"
    ast = _py_parse(src)
    e = ast[0]
    assert e["kind"]   == "export"
    assert e["format"] == "stl"


def test_parse_multiple_blocks():
    src = (
        "define a\n  cube\n    width 10\n  end\nend\n"
        "define b\n  sphere\n    radius 5\n  end\nend\n"
        "assemble c\n  place a at origin\nend"
    )
    ast = _py_parse(src)
    assert len(ast) == 3
    kinds = [item["kind"] for item in ast]
    assert kinds == ["define", "define", "assemble"]


def test_parse_primitive_with_move():
    src = "define p\n  cube\n    width 10\n    move z 5\n  end\nend"
    ast = _py_parse(src)
    prim = ast[0]["primitives"][0]
    assert ("z", 5.0) in prim["moves"]


# ── Evaluator integration tests (require _core) ───────────────────────────────

@skip_no_core
def test_eval_define_cube():
    from polyhedra._polyh_eval import _eval_define
    block = {
        "kind": "define", "name": "box", "units": "mm",
        "primitives": [{"kind": "cube", "props": {"width": 10, "depth": 10, "height": 10}, "moves": [], "units": None}],
        "manips": [],
    }
    obj = _eval_define(block)
    assert obj.distance(0, 0, 0) < 0.0


@skip_no_core
def test_eval_define_sphere():
    from polyhedra._polyh_eval import _eval_define
    block = {
        "kind": "define", "name": "ball", "units": "mm",
        "primitives": [{"kind": "sphere", "props": {"radius": 5}, "moves": [], "units": None}],
        "manips": [],
    }
    obj = _eval_define(block)
    assert obj.distance(0, 0, 0) < 0.0


@skip_no_core
def test_eval_define_with_fillet():
    from polyhedra._polyh_eval import _eval_define
    block = {
        "kind": "define", "name": "c", "units": "mm",
        "primitives": [{"kind": "cube", "props": {"width": 10, "depth": 10, "height": 10}, "moves": [], "units": None}],
        "manips": [{"op": "fillet", "value": 1.0}],
    }
    obj = _eval_define(block)
    assert isinstance(obj.__repr__(), str)


@skip_no_core
def test_eval_define_missing_primitive_raises():
    from polyhedra._polyh_eval import _eval_define
    block = {
        "kind": "define", "name": "empty", "units": "mm",
        "primitives": [], "manips": [],
    }
    with pytest.raises(ValueError, match="no primitive"):
        _eval_define(block)


@skip_no_core
def test_eval_assemble_place_and_cut():
    from polyhedra._polyh_eval import _eval_assemble
    from pathlib import Path
    defines = {
        "box": {"kind": "define", "name": "box", "units": "mm",
                "primitives": [{"kind": "cube", "props": {"width": 10, "depth": 10, "height": 10}, "moves": [], "units": None}],
                "manips": []},
        "hole": {"kind": "define", "name": "hole", "units": "mm",
                 "primitives": [{"kind": "sphere", "props": {"radius": 3}, "moves": [], "units": None}],
                 "manips": []},
    }
    asm_block = {
        "kind": "assemble", "name": "test", "units": "mm",
        "ops": [
            {"op": "place", "name": "box",  "at": (0.0, 0.0, 0.0)},
            {"op": "cut",   "name": "hole", "at": (0.0, 0.0, 0.0)},
        ],
    }
    obj = _eval_assemble(asm_block, defines, Path("."))
    # Centre was occupied by sphere → cut → now outside
    assert obj.distance(0, 0, 0) > 0.0


@skip_no_core
def test_eval_assemble_unknown_name_raises():
    from polyhedra._polyh_eval import _eval_assemble
    from pathlib import Path
    block = {
        "kind": "assemble", "name": "a", "units": "mm",
        "ops": [{"op": "place", "name": "does_not_exist", "at": (0, 0, 0)}],
    }
    with pytest.raises(ValueError, match="not defined"):
        _eval_assemble(block, {}, Path("."))
