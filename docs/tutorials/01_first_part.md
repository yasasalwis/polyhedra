# Tutorial 1 — Your First Part: A Simple Bracket

In this tutorial you will build a real mechanical part from scratch: an L-shaped mounting bracket with four M3 clearance holes and filleted edges. You will see the Python API and the `.polyh` file approach side by side, and export the finished part to both STL and GLB.

By the end you will be comfortable with:

- Creating primitive shapes with `ph.Object`
- Positioning shapes with `.translate()`
- Subtracting geometry with `.difference()` and `ph.Assembly`
- Rounding edges with `.fillet()`
- Exporting to multiple formats

---

## What We Are Building

An L-bracket has two faces at 90 degrees:

- A flat **base plate** that sits on a surface (60 × 40 × 5 mm)
- A **vertical wall** that stands upright (60 × 5 × 40 mm)
- Four **M3 clearance holes** through the base plate for mounting screws

---

## Step 1 — The Base Plate

Start with the simplest shape: a rectangular box.

```python
import polyhedra as ph

base = ph.Object(ph.CUBE, 60, 40, 5)
base.render("step1_base.stl", resolution=32)
```

`ph.Object(ph.CUBE, width, depth, height)` creates an axis-aligned box centred at the origin. With a 5 mm height, this sits from Z = -2.5 to Z = +2.5.

---

## Step 2 — The Vertical Wall

The wall is another box, but it needs to stand upright on top of the base plate. Since both shapes are centred at the origin by default, we must move the wall up so its bottom face aligns with the top face of the base.

- Base plate top face: Z = +2.5
- Wall height: 40 mm, so its centre is at Z = 2.5 + 20 = 22.5

We also need to push the wall to the back of the base plate so it forms an L:

- Wall depth: 5 mm, so its centre is at Y = (40/2) - (5/2) = 17.5

```python
wall = ph.Object(ph.CUBE, 60, 5, 40).translate(0, 17.5, 22.5)
```

To check both shapes together before adding holes:

```python
preview = base.union(wall)
preview.render("step2_lshape.stl", resolution=32)
```

---

## Step 3 — Mounting Holes

M3 screws need a 3.2 mm clearance hole (radius 1.6 mm). The hole needs to pass completely through the 5 mm base plate, so we make it 20 mm tall to ensure a clean cut regardless of position.

```python
hole = ph.Object(ph.CYLINDER, 1.6, 20)
```

Place holes at four corners of the base plate. The base plate is 60 × 40 mm, centred at the origin, so the corners are near (±25, ±15, 0):

```python
with_holes = (
    base
    .difference(hole.translate(-25, -15, 0))
    .difference(hole.translate( 25, -15, 0))
    .difference(hole.translate(-25,  15, 0))
    .difference(hole.translate( 25,  15, 0))
)
with_holes.render("step3_holes.stl", resolution=32)
```

The `-` operator is a shortcut for `.difference()`:

```python
with_holes = (
    base
    - hole.translate(-25, -15, 0)
    - hole.translate( 25, -15, 0)
    - hole.translate(-25,  15, 0)
    - hole.translate( 25,  15, 0)
)
```

---

## Step 4 — Assembly with ph.Assembly

When a part has multiple components that all need to be combined, `ph.Assembly` keeps the code readable and lets you mix `place` (union) and `cut` (difference) operations in one fluent chain.

```python
bracket = (
    ph.Assembly("bracket")
    .place(base)
    .place(wall)
    .cut(hole.translate(-25, -15, 0))
    .cut(hole.translate( 25, -15, 0))
    .cut(hole.translate(-25,  15, 0))
    .cut(hole.translate( 25,  15, 0))
)
```

`ph.Assembly` is equivalent to the `.polyh` `assemble` block. Under the hood it builds the same SDF tree as chained `.union()` and `.difference()` calls.

---

## Step 5 — Fillets

Sharp edges break printed parts and look unfinished. `.fillet(radius)` rounds every edge and corner simultaneously using an exact SDF offset — no mesh post-processing, no artefacts.

```python
result = bracket.to_object().fillet(1.5)
```

> **Note:** `.fillet()` is called on an `Object`, not on an `Assembly`. Call `.to_object()` first to collapse the assembly, then apply the fillet to the combined solid.

The fillet radius (1.5 mm) should be smaller than the thinnest wall of the part (5 mm base plate). A radius larger than half the wall thickness can cause geometry to disappear.

---

## Step 6 — Export

Export to both STL (for slicers and CAM) and GLB (for web preview or Godot):

```python
result.render("bracket.stl", resolution=64)
result.render("bracket.glb", resolution=64)
```

`.render()` returns `self`, so you can chain:

```python
result.render("bracket.stl", resolution=64).render("bracket.glb", resolution=64)
```

---

## Complete Python Script

```python
import polyhedra as ph

# Primitives
base = ph.Object(ph.CUBE, 60, 40, 5)
wall = ph.Object(ph.CUBE, 60, 5, 40).translate(0, 17.5, 22.5)
hole = ph.Object(ph.CYLINDER, 1.6, 20)

# Assembly
bracket = (
    ph.Assembly("bracket")
    .place(base)
    .place(wall)
    .cut(hole.translate(-25, -15, 0))
    .cut(hole.translate( 25, -15, 0))
    .cut(hole.translate(-25,  15, 0))
    .cut(hole.translate( 25,  15, 0))
)

# Finish and export
result = bracket.to_object().fillet(1.5)
result.render("bracket.stl", resolution=64)
result.render("bracket.glb", resolution=64)

print("Done. Check bracket.stl and bracket.glb.")
```

---

## The Same Bracket in .polyh

The `.polyh` DSL expresses the same geometry without Python. Each `define` block is a reusable named object; the `assemble` block combines them.

```
# bracket.polyh

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
  file   bracket.stl
end
```

Compile from the CLI:

```bash
polyhedra -c bracket.polyh -o stl
```

Or export multiple formats in one pass:

```bash
polyhedra -c bracket.polyh -o stl,glb --quality high
```

### Python + .polyh

You can also load the file from Python and apply additional operations before rendering:

```python
import polyhedra as ph

# compile_polyh evaluates the assemble block and returns bytes
data = ph.compile_polyh("bracket.polyh", output="stl", resolution=64)
with open("bracket.stl", "wb") as f:
    f.write(data)
```

Or load a single `define` block as an `Object`:

```python
obj = ph.load("bracket.polyh")   # returns the first define block
obj.render("base_only.stl", resolution=32)
```

---

## Reviewing the Result

Verify the geometry programmatically before writing to disk:

```python
# Centre of the base plate should be solid
assert result.distance(0, 0, 0) < 0.0

# Centre of an M3 hole should be empty
assert result.distance(-25, -15, 0) > 0.0

print("Geometry checks passed.")
```

`distance(x, y, z)` evaluates the signed distance function:
- Negative = inside the solid
- Zero = on the surface
- Positive = outside (in air)

---

## What to Try Next

- Change the fillet radius and compare results at `resolution=16` (fast) vs `resolution=64` (detailed).
- Add a chamfer instead of a fillet: replace `.fillet(1.5)` with `.chamfer(1.5)`.
- Make the bracket parametric by wrapping the code in a function — see [Tutorial 6: Python Workflow](06_python_workflow.md).
- Learn how to combine multiple brackets and other parts into a larger assembly — see [Tutorial 2: Assemblies](02_assemblies.md).
