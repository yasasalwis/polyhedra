# polyhedra

Plain-English 3D geometry compiler. Write geometry in readable text, get precision mesh files out.

Designed primarily as training data for AI models — so LLMs can learn to generate engineering-grade 3D parts from natural language. The `.polyh` language is intentionally minimal and unambiguous.

```
polyhedra -c bracket.polyh -o stl,glb --quality high
```

---

## How it works

The engine uses **Signed Distance Functions (SDF)** instead of mesh-based modeling.

- Every shape is a mathematical function: `f(x,y,z) → distance to surface`
- Boolean operations (union, difference, intersection) are 3 lines of math
- Fillets use `smooth_min` — no mesh post-processing required
- Meshing uses **Dual Contouring** — an adaptive octree that skips empty space and preserves sharp edges

Result: exact geometry, no floating-point mesh glitches, no non-manifold faces.

---

## Installation

**Requirements:** Rust 1.94+, Python ≥ 3.9, maturin 1.12+

```bash
# 1. Clone
git clone https://github.com/yourname/polyhedra
cd polyhedra

# 2. Build the Rust extension (first time / after Rust changes)
pip install maturin
python -m maturin develop --features python

# 3. Install the Python package in editable mode
pip install -e python/
```

**CLI binary** (no Python required):

```bash
cargo build --release --bin polyhedra
# binary at: target/release/polyhedra
```

---

## Three ways to use it

### 1. `.polyh` files + CLI

Write geometry in `.polyh` files. Compile with the CLI.

```
polyhedra -c bracket.polyh -o stl
polyhedra -c main.polyh -o stl,glb --quality high --dir ./output
polyhedra -c main.polyh --validate
polyhedra --list bracket.polyh
polyhedra -c main.polyh -o stl --watch
```

See the [Language Reference](docs/language-reference.md) for the full `.polyh` syntax.

### 2. Python native API

Build geometry entirely in Python — no `.polyh` files needed.

```python
import polyhedra as ph

# Primitives
cube   = ph.Object(ph.CUBE,     60, 40, 20)   # width, depth, height
sphere = ph.Object(ph.SPHERE,   10)            # radius
cyl    = ph.Object(ph.CYLINDER,  5, 30)        # radius, height
donut  = ph.Object(ph.TORUS,    20,  4)        # major, minor radius

# CSG operations (chainable)
part = (
    ph.Object(ph.CUBE, 60, 40, 20)
    .difference(ph.Object(ph.CYLINDER, 4, 25))   # drill a hole
    .fillet(1.5)                                  # round all edges
    .translate(0, 0, 10)                          # move up 10 mm
    .render("part.stl")                           # write file
)

# Operator shortcuts
combined = cube + sphere    # union
cut_part = cube - sphere    # difference
overlap  = cube & sphere    # intersection
```

See the [Python API Reference](docs/python-api.md).

### 3. Python + `.polyh` files

Load `.polyh` files from Python code.

```python
import polyhedra as ph

# Load the first define block as an Object
bracket = ph.load("bracket.polyh")
bracket.fillet(1.5).render("bracket_filleted.stl")

# Full compile (assemble + export block)
data = ph.compile_polyh("main.polyh", output="glb", resolution=64)
with open("output.glb", "wb") as f:
    f.write(data)

# Syntax check only
errors = ph.validate("bracket.polyh")
if errors:
    print("\n".join(errors))
```

---

## Tech stack

| Layer | Technology | Version |
|---|---|---|
| Kernel | Rust | 1.94.0 |
| Python bindings | PyO3 + maturin | 0.24 / 1.12.6 |
| Parser | pest PEG | 2.x |
| Math / SIMD | glam | 0.29 |
| Parallelism | rayon | 1.10 |
| File watcher | notify | 6.x |
| Output formats | STL, OBJ, PLY, GLB | — |

---

## Project layout

```
polyhedra/
├── rust/                  Rust crate (_core)
│   ├── src/
│   │   ├── sdf/           SDF primitives + operations
│   │   ├── manipulations/ Offset, Shell, Mirror, Repeat, Elongate
│   │   ├── mesher/        Dual Contouring + QEF solver
│   │   ├── export/        STL, OBJ, PLY, GLB writers
│   │   ├── parser/        .polyh PEG grammar + AST + evaluator
│   │   ├── sketch/        2D SDFs + Extrude + Revolve
│   │   ├── python.rs      PyO3 bindings
│   │   └── bin/
│   │       └── polyhedra.rs   CLI entry point
│   └── tests/             Rust integration tests
├── python/
│   └── polyhedra/         Python package
│       ├── object.py      Object class
│       ├── assembly.py    Assembly class
│       ├── io.py          load / compile_polyh / validate
│       ├── constants.py   Shape constants + plane helpers
│       └── _polyh_eval.py Pure-Python .polyh mini-parser
├── examples/              .polyh example files
└── docs/                  Full documentation
```

---

## CLI reference

```
polyhedra [OPTIONS]

Options:
  -c, --compile <FILE>    Input .polyh file
  -o, --output <FORMAT>   Output format(s): stl, obj, glb, ply, all
                          Comma-separated for multiple: stl,glb
  -q, --quality <LEVEL>   low (16vox) | medium (32vox, default) |
                          high (64vox) | ultra (128vox)
  -d, --dir <DIR>         Output directory (default: same as input)
      --validate          Syntax check only — no geometry compiled
  -w, --watch             Auto-recompile on file save (Ctrl+C to stop)
      --list <FILE>       List all define/assemble blocks in a file
      --stats             Print timing + triangle count after compile
  -j, --threads <N>       Override thread count (default: all cores)
  -h, --help              Print help
  -V, --version           Print version
```

---

## Supported primitives

| Name | Constant | Arguments |
|---|---|---|
| Box / Cube | `ph.CUBE` | `width, depth, height` |
| Sphere | `ph.SPHERE` | `radius` |
| Cylinder | `ph.CYLINDER` | `radius, height` |
| Cone / Frustum | `ph.CONE` | `base_radius, top_radius, height` |
| Torus | `ph.TORUS` | `major_radius, minor_radius` |
| Pyramid | `ph.PYRAMID` | `base_width, base_depth, height` |
| Prism (n-gon) | `ph.PRISM` | `sides, flat_to_flat, height` |

All dimensions are in **millimetres** unless a `units` statement overrides them.

---

## Running tests

```bash
# Rust (395 tests)
cargo test

# Python (81 tests — requires maturin develop first)
python -m maturin develop --features python
python -m pytest python/tests/ -v
```
