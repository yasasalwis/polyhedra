# Tutorial 6 — Python Workflow: Parametric Design

The Python API lets you combine the geometric power of polyhedra with Python's full expressiveness: variables, functions, loops, conditionals, and external data. This tutorial shows how to drive geometry parametrically, build reusable part factories, and batch-export families of parts.

Topics covered:

- Using Python variables to define dimensions
- Building a reusable part factory function
- Generating a family of parts (multiple sizes)
- Batch export to multiple formats
- Combining Python logic with polyhedra geometry
- Example: parametric standoff (configurable OD, ID, height)
- Example: a family of L-brackets (S, M, L sizes)

---

## Using Python Variables to Define Dimensions

The simplest form of parametric design: replace hardcoded numbers with named variables at the top of your script.

```python
import polyhedra as ph

# --- Parameters ---------------------------------------------------------------
BASE_W  = 60     # base plate width (mm)
BASE_D  = 40     # base plate depth (mm)
BASE_H  =  5     # base plate height (mm)
WALL_H  = 40     # vertical wall height (mm)
WALL_T  =  5     # vertical wall thickness (mm)
HOLE_R  =  1.6   # M3 clearance hole radius (mm)
FILLET  =  1.5   # edge fillet radius (mm)

# --- Geometry -----------------------------------------------------------------
base = ph.Object(ph.CUBE, BASE_W, BASE_D, BASE_H)
wall = ph.Object(ph.CUBE, BASE_W, WALL_T, WALL_H).translate(
    0,
    (BASE_D - WALL_T) / 2,
    (BASE_H + WALL_H) / 2
)
hole = ph.Object(ph.CYLINDER, HOLE_R, BASE_H + 10)

asm = (
    ph.Assembly("bracket")
    .place(base)
    .place(wall)
    .cut(hole.translate(-25, -15, 0))
    .cut(hole.translate( 25, -15, 0))
    .cut(hole.translate(-25,  15, 0))
    .cut(hole.translate( 25,  15, 0))
)

result = asm.to_object().fillet(FILLET)
result.render("bracket.stl", resolution=64)
```

Changing `BASE_W = 80` at the top automatically updates all downstream geometry. No need to hunt for numbers scattered through the code.

---

## Building a Reusable Part Factory Function

Wrap the geometry in a function that accepts parameters and returns an `Object`. This lets you call the same code with different arguments to produce different sizes of the same part.

```python
import polyhedra as ph

def l_bracket(base_w, base_d, base_h, wall_h, wall_t, hole_r=1.6, fillet_r=1.5):
    """
    Build an L-shaped mounting bracket.

    Parameters
    ----------
    base_w   : float  — base plate width (mm)
    base_d   : float  — base plate depth (mm)
    base_h   : float  — base plate height (mm)
    wall_h   : float  — vertical wall height (mm)
    wall_t   : float  — vertical wall thickness (mm)
    hole_r   : float  — mounting hole radius (mm, default M3 clearance)
    fillet_r : float  — edge fillet radius (mm)

    Returns
    -------
    ph.Object
    """
    base = ph.Object(ph.CUBE, base_w, base_d, base_h)
    wall = ph.Object(ph.CUBE, base_w, wall_t, wall_h).translate(
        0,
        (base_d - wall_t) / 2,
        (base_h + wall_h) / 2
    )
    hole = ph.Object(ph.CYLINDER, hole_r, base_h + 10)

    # Hole positions: inset 10 mm from each edge
    inset_x = base_w / 2 - 10
    inset_y = base_d / 2 - 10

    asm = (
        ph.Assembly("bracket")
        .place(base)
        .place(wall)
        .cut(hole.translate(-inset_x, -inset_y, 0))
        .cut(hole.translate( inset_x, -inset_y, 0))
        .cut(hole.translate(-inset_x,  inset_y, 0))
        .cut(hole.translate( inset_x,  inset_y, 0))
    )

    return asm.to_object().fillet(fillet_r)


# Make a single bracket
part = l_bracket(60, 40, 5, 40, 5)
part.render("bracket_60x40.stl", resolution=64)
```

---

## Generating a Family of Parts

Call the factory function in a loop to generate a whole size family:

```python
import polyhedra as ph

def l_bracket(base_w, base_d, base_h, wall_h, wall_t, hole_r=1.6, fillet_r=1.5):
    # (same function as above)
    ...

# S, M, L bracket sizes
sizes = {
    "S": dict(base_w=40,  base_d=30, base_h=4,  wall_h=30, wall_t=4,  hole_r=1.1),
    "M": dict(base_w=60,  base_d=40, base_h=5,  wall_h=40, wall_t=5,  hole_r=1.6),
    "L": dict(base_w=100, base_d=60, base_h=8,  wall_h=60, wall_t=8,  hole_r=2.1),
}

for name, params in sizes.items():
    part = l_bracket(**params)
    part.render(f"bracket_{name}.stl", resolution=64)
    print(f"Exported bracket_{name}.stl")
```

---

## Example: Parametric Standoff

A standoff (PCB spacer) is a hollow cylinder with an outer diameter, inner bore, and configurable height. This is one of the most common parametric parts in electronics enclosures.

```python
import polyhedra as ph

def standoff(od, id_, height, fillet_r=0.5):
    """
    Build a cylindrical standoff.

    Parameters
    ----------
    od       : float  — outer diameter (mm)
    id_      : float  — inner bore diameter (mm)
    height   : float  — total height (mm)
    fillet_r : float  — top/bottom edge fillet (mm)

    Returns
    -------
    ph.Object
    """
    outer = ph.Object(ph.CYLINDER, od / 2, height)
    bore  = ph.Object(ph.CYLINDER, id_ / 2, height + 2)   # slightly taller to ensure clean cut
    return outer.difference(bore.translate(0, 0, -1)).fillet(fillet_r)


# Generate M3, M4, M5 standoffs at 10 mm height
for size_name, (od, id_) in [("M3", (6, 3.2)), ("M4", (8, 4.2)), ("M5", (10, 5.2))]:
    standoff(od, id_, 10).render(f"standoff_{size_name}_10mm.stl", resolution=32)
    print(f"standoff_{size_name}_10mm.stl done")
```

### Generating a full matrix

Generate standoffs for multiple sizes and heights:

```python
import polyhedra as ph

sizes = {
    "M3": (6,   3.2),
    "M4": (8,   4.2),
    "M5": (10,  5.2),
    "M6": (12,  6.4),
}

heights = [5, 10, 15, 20]

for size_name, (od, id_) in sizes.items():
    for h in heights:
        obj = standoff(od, id_, h)
        filename = f"standoff_{size_name}_{h}mm.stl"
        obj.render(filename, resolution=32)
        print(f"  {filename}")
```

This produces 16 parts (4 sizes × 4 heights) in one script run.

---

## Batch Export to Multiple Formats

Export each part to STL (for printing) and GLB (for web preview) in the same loop:

```python
import polyhedra as ph

def standoff(od, id_, height, fillet_r=0.5):
    outer = ph.Object(ph.CYLINDER, od / 2, height)
    bore  = ph.Object(ph.CYLINDER, id_ / 2, height + 2)
    return outer.difference(bore.translate(0, 0, -1)).fillet(fillet_r)


for size_name, (od, id_) in [("M3", (6, 3.2)), ("M4", (8, 4.2)), ("M5", (10, 5.2))]:
    part = standoff(od, id_, 10)
    base_name = f"standoff_{size_name}_10mm"

    # Production STL for printing
    part.render(f"{base_name}.stl", resolution=64)

    # GLB for web catalogue
    part.render(f"{base_name}.glb", resolution=64)

    # Fast preview (for CI/CD checks)
    part.render(f"{base_name}_preview.stl", resolution=16)

    print(f"{size_name}: done")
```

---

## Combining Python Logic with Geometry

Python logic can drive which geometry gets created, not just its dimensions.

### Conditional features

```python
import polyhedra as ph

def enclosure(width, depth, height, *, with_rib=True, rib_t=2.0, wall_t=2.0):
    """
    A hollow rectangular enclosure, optionally with an internal stiffening rib.
    """
    outer = ph.Object(ph.CUBE, width, depth, height)
    shell = outer.shell(wall_t)

    if with_rib:
        rib = ph.Object(ph.CUBE, width - wall_t * 2, rib_t, height - wall_t * 2)
        return shell.union(rib)

    return shell

# No rib
thin = enclosure(80, 60, 30, with_rib=False)
thin.render("enclosure_thin.stl", resolution=64)

# With rib
ribbed = enclosure(80, 60, 30, with_rib=True, rib_t=3.0)
ribbed.render("enclosure_ribbed.stl", resolution=64)
```

### Geometry from external data

Read dimensions from a CSV and generate one part per row:

```python
import csv
import polyhedra as ph

def standoff(od, id_, height):
    outer = ph.Object(ph.CYLINDER, od / 2, height)
    bore  = ph.Object(ph.CYLINDER, id_ / 2, height + 2)
    return outer.difference(bore.translate(0, 0, -1)).fillet(0.5)

with open("standoffs.csv") as f:
    for row in csv.DictReader(f):
        name   = row["name"]
        od     = float(row["od"])
        id_    = float(row["id"])
        height = float(row["height"])
        standoff(od, id_, height).render(f"{name}.stl", resolution=32)
        print(f"  {name}.stl")
```

Where `standoffs.csv` contains:

```
name,od,id,height
M3_5mm,6,3.2,5
M3_10mm,6,3.2,10
M4_10mm,8,4.2,10
M5_15mm,10,5.2,15
```

---

## Verifying Geometry Programmatically

The `.distance(x, y, z)` method evaluates the SDF at a point. Use it to write geometry assertions:

```python
import polyhedra as ph

def standoff(od, id_, height):
    outer = ph.Object(ph.CYLINDER, od / 2, height)
    bore  = ph.Object(ph.CYLINDER, id_ / 2, height + 2)
    return outer.difference(bore.translate(0, 0, -1)).fillet(0.5)

part = standoff(6, 3.2, 10)

# Centre of the bore should be empty (positive distance = outside solid)
assert part.distance(0, 0, 0) > 0.0, "Bore should be hollow at centre"

# Point in the wall should be solid (negative distance = inside solid)
assert part.distance(2.2, 0, 0) < 0.0, "Wall should be solid"

print("Geometry assertions passed.")
```

These checks are fast (microseconds each) and can be part of a CI pipeline to catch regressions when you change dimensions.

---

## Loading .polyh Files in a Python Workflow

You can mix Python parametric logic with `.polyh`-defined shapes:

```python
import polyhedra as ph

# Load a standard part defined in .polyh
m3_hole = ph.load("library/m3_clearance_hole.polyh")

# Build the rest in Python
def plate_with_holes(width, depth, thickness, hole_positions):
    plate = ph.Object(ph.CUBE, width, depth, thickness)
    asm   = ph.Assembly("plate").place(plate)
    for x, y in hole_positions:
        asm.cut(m3_hole.translate(x, y, 0))
    return asm.to_object().fillet(1.0)

positions = [(-30, -15), (30, -15), (-30, 15), (30, 15)]
part = plate_with_holes(80, 50, 5, positions)
part.render("plate.stl", resolution=64)
```

This pattern is useful for teams where hardware engineers own the `.polyh` library files, and software engineers write Python scripts that consume them.

---

## Quick Reference: Python API Essentials

```python
import polyhedra as ph

# Primitives
ph.Object(ph.CUBE,     width, depth, height)
ph.Object(ph.CYLINDER, radius, height)
ph.Object(ph.SPHERE,   radius)
ph.Object(ph.CONE,     base_radius, top_radius, height)
ph.Object(ph.TORUS,    major_radius, minor_radius)
ph.Object(ph.PYRAMID,  base_width, base_depth, height)
ph.Object(ph.PRISM,    sides, flat_to_flat, height)

# Boolean CSG
a.union(b)          # or: a + b
a.difference(b)     # or: a - b
a.intersection(b)   # or: a & b
a.smooth_union(b, blend=5.0)

# Transforms
obj.translate(x, y, z)
obj.rotate_x(deg)
obj.rotate_y(deg)
obj.rotate_z(deg)
obj.scale(factor)
obj.mirror_x()

# Manipulations
obj.fillet(r)
obj.chamfer(r)
obj.shell(t)
obj.offset(a)

# Assembly
asm = ph.Assembly("name")
asm.place(obj)        # union
asm.cut(obj)          # difference
asm.intersect(obj)
solid = asm.to_object()

# Export
obj.render("out.stl", resolution=64)
obj.render("out.glb", resolution=64)
obj.to_bytes("stl", resolution=64)

# File I/O
ph.load("part.polyh")                          # first define block → Object
ph.compile_polyh("main.polyh", output="stl")   # first assemble block → bytes
ph.validate("part.polyh")                      # syntax check → list[str]
```

---

## Next Steps

- [Tutorial 1: Your First Part](01_first_part.md) — if you skipped it, this is the foundation
- [Tutorial 2: Assemblies](02_assemblies.md) — `ph.Assembly` in depth
- [Tutorial 3: Manipulations](03_manipulations.md) — fillet, shell, and offset
- [Tutorial 4: Export Formats](04_export_formats.md) — choosing the right format and resolution
- [Python API Reference](../python-api.md) — complete reference for every class and method
