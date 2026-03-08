# Examples

Two `.polyh` example files showing the language in action.

---

## `hello.polyh` — standalone defines

Defines two reusable objects. No `assemble` block — compiles to the first define (`bracket`).

```
polyhedra -c hello.polyh -o stl --quality medium
```

**`bracket`** — an L-shaped bracket:
- 60 × 40 × 5 mm base plate
- 60 × 5 × 40 mm vertical wall (moved +5 mm in Z to sit on top of the base)
- `chamfer 1.5` applied to the union of both cubes

**`m3_hole`** — an M3 clearance hole template:
- Cylinder, radius 1.6 mm, height 20 mm

---

## `main.polyh` — assembly entry point

Imports `hello.polyh` and assembles a finished part.

```
polyhedra -c main.polyh -o stl --quality high
```

`assemble bracket_with_holes`:
- Places `bracket` at the origin
- Cuts four `m3_hole` instances at the four mounting hole positions

`export` block:
- Format: STL
- Quality: high
- Output file: `bracket_with_holes.stl`

---

## How to extend these examples

### Add another hole

In `main.polyh`, add more `cut` lines inside the `assemble` block:

```
cut m3_hole at (-25, -15, 0) pointing up
cut m3_hole at ( 25, -15, 0) pointing up
cut m3_hole at (  0,   0, 0) pointing up    # centre hole
```

### Add a fillet to the assembly

After the `cut` statements, add a manipulation:

```
assemble bracket_with_holes
  ...
  chamfer 1.0
end
```

### Change quality

```
polyhedra -c main.polyh -o stl --quality ultra
```

Quality levels: `low` (16 vox), `medium` (32 vox), `high` (64 vox), `ultra` (128 vox).

### Export multiple formats at once

```
polyhedra -c main.polyh -o stl,glb,obj --quality high --dir ./output
```

### Compile from Python

```python
import polyhedra as ph

# Compile main.polyh to GLB at high resolution
data = ph.compile_polyh("main.polyh", output="glb", resolution=64)
with open("bracket_with_holes.glb", "wb") as f:
    f.write(data)

# Load just the bracket define and add a Python-side fillet
bracket = ph.load("hello.polyh")
bracket.fillet(2.0).render("bracket_filleted.stl", resolution=64)
```

---

## Reproduce the bracket entirely in Python (no .polyh files)

```python
import polyhedra as ph

base_plate    = ph.Object(ph.CUBE, 60, 40,  5)
vertical_wall = ph.Object(ph.CUBE, 60,  5, 40).translate(0, 0, 22.5)
m3_hole       = ph.Object(ph.CYLINDER, 1.6, 20)

bracket_with_holes = (
    ph.Assembly("bracket_with_holes")
    .place(base_plate)
    .place(vertical_wall)
    .cut(m3_hole.translate(-25, -15, 0))
    .cut(m3_hole.translate( 25, -15, 0))
    .cut(m3_hole.translate(-25,  15, 0))
    .cut(m3_hole.translate( 25,  15, 0))
    .render("bracket_with_holes.stl", resolution=64)
)
```
