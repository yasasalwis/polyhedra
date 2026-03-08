# polyhedra Python API Reference

```python
import polyhedra as ph
```

All geometry operations return new `Object` instances — the original is never mutated. Methods can be chained freely.

---

## Shape constants

Used as the first argument to `ph.Object(...)`.

| Constant | Value | Shape |
|---|---|---|
| `ph.CUBE` | 0 | Axis-aligned rectangular box |
| `ph.CYLINDER` | 1 | Right cylinder |
| `ph.SPHERE` | 2 | Sphere |
| `ph.CONE` | 3 | Cone or frustum |
| `ph.TORUS` | 4 | Torus (donut) |
| `ph.PYRAMID` | 5 | Rectangular pyramid |
| `ph.PRISM` | 6 | Regular n-sided prism |

---

## `ph.Object`

The primary geometry type. Wraps a Rust SDF node.

### Construction

```python
ph.Object(shape: int, *args: float) -> Object
```

All dimensions are in **millimetres**.

| Shape | Arguments | Example |
|---|---|---|
| `ph.CUBE` | `width, depth, height` | `ph.Object(ph.CUBE, 60, 40, 20)` |
| `ph.SPHERE` | `radius` | `ph.Object(ph.SPHERE, 10)` |
| `ph.CYLINDER` | `radius, height` | `ph.Object(ph.CYLINDER, 5, 30)` |
| `ph.CONE` | `base_radius, top_radius, height` | `ph.Object(ph.CONE, 8, 0, 20)` |
| `ph.TORUS` | `major_radius, minor_radius` | `ph.Object(ph.TORUS, 20, 4)` |
| `ph.PYRAMID` | `base_width, base_depth, height` | `ph.Object(ph.PYRAMID, 10, 10, 15)` |
| `ph.PRISM` | `sides, flat_to_flat, height` | `ph.Object(ph.PRISM, 6, 12, 20)` |

For `ph.CONE`: `top_radius=0` gives a sharp tip; any value > 0 gives a flat-top frustum.
For `ph.PRISM`: `flat_to_flat` is the inscribed-circle diameter (wrench flat size).

---

### Boolean CSG operations

#### `union(other)` / `a + b`

```python
result = cube.union(sphere)
result = cube + sphere          # operator shortcut
```

Combine two objects. The result contains all volume from both.

---

#### `difference(other)` / `a - b`

```python
result = cube.difference(sphere)
result = cube - sphere
```

Subtract `other` from `self`. Removes the volume of `other`.

---

#### `intersection(other)` / `a & b`

```python
result = cube.intersection(sphere)
result = cube & sphere
```

Keep only the volume shared by both objects.

---

#### `smooth_union(other, blend=5.0)`

```python
result = a.smooth_union(b, blend=3.0)
```

Union with a smooth rounded fillet at the join. `blend` is the fillet radius in mm — higher values produce a softer, more blended join.

---

#### `smooth_difference(other, blend=5.0)`

```python
result = cube.smooth_difference(sphere, blend=2.0)
```

Difference with a smoothed chamfer at the cut edge.

---

#### `chamfer_union(other, radius=2.0)`

```python
result = a.chamfer_union(b, radius=1.5)
```

Union with a flat 45-degree chamfer where the two objects meet.

---

### Manipulations

#### `fillet(radius)`

```python
rounded = cube.fillet(2.0)
```

Round all edges and corners by `radius` mm. Uses an exact SDF offset — no mesh post-processing required.

---

#### `chamfer(radius)`

```python
bevelled = cube.chamfer(1.5)
```

Bevel all edges by `radius` mm.

---

#### `shell(thickness)`

```python
hollow = cube.shell(2.0)
```

Hollow out the object, leaving a wall of `thickness` mm. The interior becomes empty space.

```python
# Verify: centre should be outside (hollow)
hollow = ph.Object(ph.CUBE, 10, 10, 10).shell(1.0)
assert hollow.distance(0, 0, 0) > 0.0
```

---

#### `offset(amount)`

```python
grown   = obj.offset(1.0)    # grow by 1 mm
shrunk  = obj.offset(-0.5)   # shrink by 0.5 mm
```

Grow or shrink the object by `amount` mm in all directions. Positive values round convex edges; negative values trim them.

---

#### `elongate(x=0, y=0, z=0)`

```python
capsule = ph.Object(ph.SPHERE, 5).elongate(z=10)
```

Stretch the object by extruding its interior. Produces capsule-like shapes from spheres, slot extrusions from cylinders, etc.

---

### Transforms

All transforms are exact — they apply an inverse transform to the query point rather than moving mesh vertices.

#### `translate(x=0, y=0, z=0)`

```python
moved = obj.translate(10, 0, 5)
```

Move the object by `(x, y, z)` mm.

---

#### `rotate_x(degrees)` / `rotate_y(degrees)` / `rotate_z(degrees)`

```python
tilted = obj.rotate_x(45)      # tilt 45° around X axis
spun   = obj.rotate_z(90)      # spin 90° around Z axis
```

Rotate about the given axis. Angles are in degrees.

---

#### `scale(factor)`

```python
big = sphere.scale(3)          # 3× uniform scale
```

Scale uniformly about the origin.

---

#### `scale_xyz(sx, sy, sz)`

```python
squashed = obj.scale_xyz(1.0, 1.0, 0.5)   # half height
```

Non-uniform scale along each axis independently.

---

#### `mirror_x()` / `mirror_y()` / `mirror_z()`

```python
# Mirror a sphere at x=8 → creates two spheres symmetric about YZ plane
s  = ph.Object(ph.SPHERE, 3).translate(8, 0, 0)
ms = s.mirror_x()
assert ms.distance(-8, 0, 0) < 0.0   # both sides present
assert ms.distance( 8, 0, 0) < 0.0
```

Mirror across the YZ / XZ / XY plane respectively. Each mirror returns both the original and its reflection.

---

### Meshing and export

#### `mesh(*, resolution=None, refinement=None, bounds=None)`

```python
m = obj.mesh()
m = obj.mesh(resolution=64)
m = obj.mesh(refinement=0.5)    # 0.5 mm voxel size
```

Triangulate using Dual Contouring. Returns a `_core.Mesh`.

| Argument | Default | Meaning |
|---|---|---|
| `resolution` | 32 | Voxels per axis |
| `refinement` | — | Voxel size in mm (overrides `resolution`) |
| `bounds` | auto | Half-size of meshing volume in mm |

Resolution / quality guide:

| `resolution` | CLI equivalent | Use for |
|---|---|---|
| 16 | `--quality low` | Quick preview |
| 32 | `--quality medium` | General use |
| 64 | `--quality high` | Final parts |
| 128 | `--quality ultra` | Highly detailed models |

---

#### `to_bytes(fmt="stl", *, resolution=None, refinement=None, bounds=None)`

```python
data = obj.to_bytes("stl")
data = obj.to_bytes("glb", resolution=64)
```

Mesh and return the file content as `bytes`. `fmt` is one of `"stl"`, `"obj"`, `"ply"`, `"glb"`.

---

#### `render(path, *, resolution=None, refinement=None, bounds=None)`

```python
obj.render("output.stl")
obj.render("output.glb", resolution=64)
```

Mesh and write to `path`. The format is inferred from the file extension. Returns `self` for chaining.

```python
# Chain multiple renders at different quality levels
(ph.Object(ph.SPHERE, 10)
   .fillet(1)
   .render("hi.stl", resolution=64)
   .render("lo.stl", resolution=16))
```

---

#### `distance(x, y, z)`

```python
d = obj.distance(0, 0, 0)
# d < 0  → inside
# d == 0 → on surface
# d > 0  → outside
```

Evaluate the SDF at world-space point `(x, y, z)`. Useful for testing and debugging geometry.

---

### Operator overloads

| Expression | Equivalent |
|---|---|
| `a + b` | `a.union(b)` |
| `a - b` | `a.difference(b)` |
| `a & b` | `a.intersection(b)` |

---

## `ph.Assembly`

Combine multiple `Object` instances using named CSG operations. Equivalent to an `assemble` block in `.polyh`.

```python
asm = ph.Assembly("bracket")
```

### Methods

#### `place(obj)` / `join(obj)`

```python
asm.place(ph.Object(ph.CUBE, 60, 40, 5))
```

Add `obj` to the assembly using a union operation.

---

#### `cut(obj)` / `subtract(obj)`

```python
asm.cut(ph.Object(ph.CYLINDER, 4, 20))
```

Subtract `obj` from the assembly.

---

#### `intersect(obj)`

```python
asm.intersect(ph.Object(ph.SPHERE, 30))
```

Keep only the volume of the assembly that overlaps with `obj`.

---

#### `to_object()`

```python
solid = asm.to_object()
```

Collapse all steps into a single `Object`. Raises `ValueError` if the assembly is empty.

---

#### `mesh(**kwargs)` / `to_bytes(fmt, **kwargs)` / `render(path, **kwargs)`

Delegates to `to_object()` then calls the matching method. All keyword arguments are forwarded to the underlying `Object` method.

---

### Full example

```python
import polyhedra as ph

bracket = (
    ph.Assembly("bracket")
    .place(ph.Object(ph.CUBE, 60, 40, 5))                           # base plate
    .place(ph.Object(ph.CUBE, 60, 5, 40).translate(0, 17.5, 22.5)) # vertical wall
    .cut(ph.Object(ph.CYLINDER, 1.6, 20).translate(-25, -15, 0))   # M3 hole
    .cut(ph.Object(ph.CYLINDER, 1.6, 20).translate( 25, -15, 0))
    .cut(ph.Object(ph.CYLINDER, 1.6, 20).translate(-25,  15, 0))
    .cut(ph.Object(ph.CYLINDER, 1.6, 20).translate( 25,  15, 0))
    .render("bracket.stl", resolution=64)
)
```

---

## File I/O

### `ph.load(path)`

```python
obj = ph.load("bracket.polyh")
```

Parse a `.polyh` file and return the **first `define` block** as an `Object`. Raises `FileNotFoundError` if the file does not exist.

---

### `ph.compile_polyh(path, *, output="stl", resolution=None, refinement=None, bounds=None)`

```python
data = ph.compile_polyh("main.polyh", output="glb", resolution=64)
with open("output.glb", "wb") as f:
    f.write(data)
```

Parse the file, evaluate the **first `assemble` block**, and return the mesh as `bytes`. Raises `ValueError` if the file contains no `assemble` block.

---

### `ph.validate(path)`

```python
errors = ph.validate("bracket.polyh")
if errors:
    for msg in errors:
        print(msg)
else:
    print("syntax ok")
```

Check `.polyh` syntax without meshing. Returns a list of error strings (empty = valid).

---

## Construction planes (`ph.planes`)

Used to define sketch planes (reserved for the Sketch API).

```python
ph.planes.XY               # Z-normal plane at Z = 0
ph.planes.YZ               # X-normal plane at X = 0
ph.planes.XZ               # Y-normal plane at Y = 0
ph.planes.Origin           # alias for XY

ph.planes.XY.offset(10)    # XY plane shifted to Z = +10 mm
ph.planes.XY.offset(-5)    # XY plane shifted to Z = -5 mm

ph.planes.custom(point=(0, 0, 5), normal=(0, 0, 1))
```

---

## Complete worked example

```python
import polyhedra as ph

# ── Parametric mounting bracket ───────────────────────────────────────────────

BASE_W,  BASE_D,  BASE_H  = 80, 50, 6
WALL_H = 50
WALL_T =  6
HOLE_R =  2.0    # M4 clearance
FILLET =  2.0

# Parts
base = ph.Object(ph.CUBE, BASE_W, BASE_D, BASE_H)
wall = ph.Object(ph.CUBE, BASE_W, WALL_T, WALL_H).translate(
    0, (BASE_D - WALL_T) / 2, (BASE_H + WALL_H) / 2
)
hole = ph.Object(ph.CYLINDER, HOLE_R, BASE_H + 4)

# Hole positions
hole_positions = [
    (-30, -18, 0), (30, -18, 0),
    (-30,  18, 0), (30,  18, 0),
]

bracket = (
    ph.Assembly("bracket")
    .place(base)
    .place(wall)
)
for x, y, z in hole_positions:
    bracket.cut(hole.translate(x, y, z))

result = bracket.to_object().fillet(FILLET)

result.render("bracket.stl",  resolution=64)
result.render("bracket.glb",  resolution=64)

# Check it's solid at the centre
assert result.distance(0, 0, 3) < 0.0
print(f"Bounds: ≈{result._bounds:.1f} mm")
```
