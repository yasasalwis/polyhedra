# Quick Start — Up and Running in 5 Minutes

This guide gets you from a fresh checkout to a rendered STL in five minutes. You will learn three ways to use polyhedra: the CLI, the Python native API, and Python combined with `.polyh` files.

---

## 1. Installation

polyhedra ships as a Python extension backed by a Rust SDF kernel. Build it in-place using [maturin](https://www.maturin.rs/):

```bash
pip install maturin
maturin develop --release   # compile the Rust extension and install into the current venv
pip install -e python/      # install the Python wrapper in editable mode
```

Verify the install:

```python
import polyhedra as ph
print(ph.__version__)
```

---

## 2. Hello, polyhedra

Three one-liners, three different entry points — all produce an STL file.

### CLI

```bash
polyhedra -c examples/hello.polyh -o stl
```

This compiles `hello.polyh`, meshes the first `assemble` block, and writes `output.stl` next to the source file. Add `--quality high` for production output.

### Python native API

```python
import polyhedra as ph
cube = ph.Object(ph.CUBE, 60, 40, 20)   # 60 × 40 × 20 mm box
cube.render("hello.stl")
```

`ph.Object` takes a shape constant followed by the shape's dimensions (all in millimetres). `.render()` meshes and writes the file in one step.

### Python + .polyh file

```python
import polyhedra as ph
ph.load("examples/hello.polyh").render("hello.stl")
```

`ph.load()` parses the file and returns the first `define` block as a Python `Object`. You can then apply any Python API method to it before rendering.

---

## 3. Your First Bracket

A real part in under 20 lines. This builds an L-shaped mounting bracket with four M3 clearance holes.

```python
import polyhedra as ph

# --- geometry -----------------------------------------------------------------
base = ph.Object(ph.CUBE, 60, 40, 5)           # base plate: 60 × 40 × 5 mm
wall = ph.Object(ph.CUBE, 60, 5, 40).translate( # vertical wall
    0, 17.5, 22.5                               # sit it on top of the base
)

# M3 clearance hole: radius 1.6 mm, tall enough to pass all the way through
hole = ph.Object(ph.CYLINDER, 1.6, 20)

# --- assemble -----------------------------------------------------------------
bracket = (
    ph.Assembly("bracket")
    .place(base)
    .place(wall)
    .cut(hole.translate(-25, -15, 0))
    .cut(hole.translate( 25, -15, 0))
    .cut(hole.translate(-25,  15, 0))
    .cut(hole.translate( 25,  15, 0))
)

# --- finish and export --------------------------------------------------------
result = bracket.to_object().fillet(1.5)        # round all edges 1.5 mm
result.render("bracket.stl", resolution=64)     # production-quality STL
result.render("bracket.glb", resolution=64)     # GLB for web preview
```

Run it:

```bash
python bracket.py
```

You should see `bracket.stl` and `bracket.glb` appear in the current directory.

> **Tip:** Use `resolution=16` while iterating to keep render times under a second, then switch to `resolution=64` for final output.

---

## 4. Your First .polyh File

The `.polyh` DSL lets you describe geometry in plain English. It is readable by humans and LLMs alike, and compiles to the same SDF kernel as the Python API.

Create `my_bracket.polyh`:

```
# my_bracket.polyh
# A simple L-shaped mounting bracket.

define base_plate
  units mm
  cube
    width  60
    depth  40
    height  5
  end
end

define vertical_wall
  units mm
  cube
    width  60
    depth   5
    height 40
    move z 22.5
  end
end

define m3_hole
  units mm
  cylinder
    radius  1.6
    height 20
  end
end

assemble bracket
  units mm

  place base_plate    at origin
  place vertical_wall at origin

  cut m3_hole at (-25, -15, 0)
  cut m3_hole at ( 25, -15, 0)
  cut m3_hole at (-25,  15, 0)
  cut m3_hole at ( 25,  15, 0)

  fillet 1.5
end

export
  format stl
  quality high
  file   my_bracket.stl
end
```

Compile it:

```bash
polyhedra -c my_bracket.polyh -o stl
```

Or from Python:

```python
import polyhedra as ph
data = ph.compile_polyh("my_bracket.polyh", output="stl", resolution=64)
with open("my_bracket.stl", "wb") as f:
    f.write(data)
```

### Key rules

- Blocks open with their keyword (`define`, `assemble`, `export`) and close with `end`.
- Indentation is cosmetic — only the keywords matter.
- `#` starts a comment to the end of the line.
- Dimensions are always literal numbers; the language has no variables or arithmetic.
- All dimensions default to millimetres unless a `units` statement is present.

---

## 5. Next Steps

You are ready to build real parts. Here is where to go next:

| Topic | File |
|---|---|
| Step-by-step bracket tutorial | [tutorials/01_first_part.md](tutorials/01_first_part.md) |
| Combining parts into assemblies | [tutorials/02_assemblies.md](tutorials/02_assemblies.md) |
| Fillets, chamfers, shells, and offsets | [tutorials/03_manipulations.md](tutorials/03_manipulations.md) |
| Exporting to STL, OBJ, PLY, and GLB | [tutorials/04_export_formats.md](tutorials/04_export_formats.md) |
| Writing .polyh files from scratch | [tutorials/05_polyh_language.md](tutorials/05_polyh_language.md) |
| Parametric design with Python | [tutorials/06_python_workflow.md](tutorials/06_python_workflow.md) |
| Complete language reference | [language-reference.md](language-reference.md) |
| Complete Python API reference | [python-api.md](python-api.md) |
