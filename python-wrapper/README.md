# polyhedra — Python package

Pure-Python wrapper around the `_core` Rust extension. Provides the ergonomic `Object` and `Assembly` API, the `.polyh` mini-parser, and file I/O helpers.

## Package layout

```
polyhedra/
├── __init__.py         Public API surface + ImportWarning when _core is absent
├── object.py           Object class — all CSG ops, transforms, manipulations, export
├── assembly.py         Assembly class — multi-step CSG builder
├── constants.py        Shape constants (CUBE, SPHERE, ...) + ph.planes namespace
├── io.py               load(), compile_polyh(), validate()
└── _polyh_eval.py      Pure-Python .polyh tokeniser + parser + evaluator
```

## Development setup

```bash
# From project root
pip install maturin
python -m maturin develop --features python   # build _core extension
pip install -e python-wrapper/                         # install polyhedra in editable mode

# Run tests
python -m pytest python/tests/ -v
```

## How `Object` works

`Object` holds two fields:
- `_node` — a `_core.SdfNode` (the Rust SDF tree)
- `_bounds` — a float hint for the object's approximate half-extent in mm

Every method that produces geometry creates a new `Object` wrapping a new `_node`.
The bounds hint propagates through operations (union = max, difference = self, scale = multiply) and is used by `mesh()` to size the meshing volume automatically.

## The pure-Python mini-parser (`_polyh_eval.py`)

Used by `ph.load()` and `ph.validate()` to parse `.polyh` files without needing the full Rust parser (which is only available through the CLI binary). It implements:

1. `_tokenise(source)` — strip comments, expand parentheses, split on whitespace
2. `_py_parse(source)` — state-machine parser producing a list of AST dicts
3. `_eval_define(block)` → `Object`
4. `_eval_assemble(block, defines, base_path)` → `Object`

The Rust parser (`_core` parser module) is not yet exposed through PyO3 — `_polyh_eval.py` is the current implementation for Python-side `.polyh` support.
