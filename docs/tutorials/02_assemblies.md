# Tutorial 2 — Assemblies: Combining Multiple Parts

In this tutorial you will learn how to combine several independently-defined parts into a single solid. The key ideas are the `define` / `assemble` distinction in `.polyh` and the `ph.Assembly` class in Python. By the end you will have built two multi-part examples: a pipe fitting and a heat sink.

Topics covered:

- The difference between `define` (reusable part) and `assemble` (composition)
- Python `ph.Assembly` — `.place()`, `.cut()`, `.join()`, `.intersect()`
- The `.polyh` `assemble` block syntax
- Positioning parts: `at origin` vs `at (x, y, z)`
- Reusing the same `define` at multiple positions
- Two worked examples: pipe fitting and heat sink

---

## define vs assemble

### `define` — a named, reusable shape

A `define` block describes a single self-contained object. It has no knowledge of any other part. You can think of it as a function that produces a solid.

In `.polyh`:

```
define pipe_body
  units mm
  cylinder
    radius 15
    height 80
  end
end
```

In Python:

```python
import polyhedra as ph

pipe_body = ph.Object(ph.CYLINDER, 15, 80)
```

### `assemble` — a composition of named objects

An `assemble` block (or `ph.Assembly`) takes previously-defined parts and combines them using boolean operations. It is the only place where parts are positioned relative to each other.

In `.polyh`:

```
assemble pipe_fitting
  place pipe_body at origin
  place flange    at (0, 0,  40)
  place flange    at (0, 0, -40)
  cut   bore      at origin
end
```

In Python:

```python
fitting = (
    ph.Assembly("pipe_fitting")
    .place(pipe_body)
    .place(flange.translate(0, 0,  40))
    .place(flange.translate(0, 0, -40))
    .cut(bore)
)
```

The key rule: **every `place`/`cut`/`intersect` in an assembly positions an already-defined object.** You build objects in `define` blocks, then compose them in `assemble`.

---

## ph.Assembly Operations

| Method | Effect |
|---|---|
| `.place(obj)` | Union — add `obj` to the assembly |
| `.join(obj)` | Alias for `.place()` |
| `.cut(obj)` | Difference — subtract `obj` |
| `.subtract(obj)` | Alias for `.cut()` |
| `.intersect(obj)` | Keep only the volume shared with `obj` |
| `.to_object()` | Collapse all steps into a single `Object` |

All methods return the assembly itself, so calls can be chained.

After building the assembly, call `.to_object()` to get an `Object` that you can fillet, shell, or render:

```python
solid = my_asm.to_object()
solid.fillet(1.0).render("part.stl", resolution=64)
```

`ph.Assembly` also has `.render()`, `.to_bytes()`, and `.mesh()` methods that call `.to_object()` automatically:

```python
my_asm.render("part.stl", resolution=64)
```

---

## Positioning Parts

### `at origin`

Places the object with its own origin coincident with the assembly's origin. Use this for the first part you place, or when a part is already in the correct position by design.

`.polyh`:

```
place pipe_body at origin
```

Python:

```python
asm.place(pipe_body)   # no translate needed
```

### `at (x, y, z)`

Shifts the object so that its origin lands at `(x, y, z)` in the assembly coordinate space.

`.polyh`:

```
place flange at (0, 0, 40)
```

Python:

```python
asm.place(flange.translate(0, 0, 40))
```

In Python, translation is applied to the object before passing it to the assembly. In `.polyh`, the `at (x, y, z)` clause does the same thing automatically.

---

## Example 1 — Pipe Fitting

A pipe fitting has a cylindrical body, two circular flanges at each end, and a through-bore.

### Python

```python
import polyhedra as ph

# Parts
pipe_body = ph.Object(ph.CYLINDER, 15, 80)   # outer tube: R=15, L=80
flange    = ph.Object(ph.CYLINDER, 25,  8)   # flange disc: R=25, T=8
bore      = ph.Object(ph.CYLINDER, 10, 90)   # through-bore: R=10, slightly longer than body

fitting = (
    ph.Assembly("pipe_fitting")
    .place(pipe_body)
    .place(flange.translate(0, 0,  44))   # top flange: sits on top of tube
    .place(flange.translate(0, 0, -44))   # bottom flange: mirrors top
    .cut(bore)                            # drill through the whole assembly
)

result = fitting.to_object().fillet(1.0)
result.render("pipe_fitting.stl", resolution=64)
```

### .polyh

```
# pipe_fitting.polyh

define pipe_body
  units mm
  cylinder
    radius 15
    height 80
  end
end

define flange
  units mm
  cylinder
    radius 25
    height  8
  end
end

define bore
  units mm
  cylinder
    radius 10
    height 90
  end
end

assemble pipe_fitting
  units mm

  place pipe_body at origin
  place flange    at (0, 0,  44)
  place flange    at (0, 0, -44)

  cut bore at origin

  fillet 1.0
end

export
  format stl
  quality high
  file   pipe_fitting.stl
end
```

Note how the same `flange` define is placed twice at different Z positions. In Python this is `flange.translate(0, 0, 44)` and `flange.translate(0, 0, -44)`. In `.polyh` it is `place flange at (0, 0, 44)` and `place flange at (0, 0, -44)`. The `define` is defined once and reused twice.

---

## Example 2 — Heat Sink

A heat sink has a flat base plate and an array of thin cooling fins standing upright on top of it.

### Python

```python
import polyhedra as ph

# The base plate
base = ph.Object(ph.CUBE, 60, 40, 5)

# A single fin: 2 mm wide, 40 mm deep, 20 mm tall
# Fins sit on top of the base (base top = Z+2.5, fin centre = Z+2.5+10 = 12.5)
fin = ph.Object(ph.CUBE, 2, 40, 20).translate(0, 0, 12.5)

# Build the heat sink: base + 7 fins spaced 8 mm apart
# Fins span from X = -24 to X = +24 (7 fins × 8 mm spacing)
asm = ph.Assembly("heat_sink").place(base)

for i in range(7):
    x = -24 + i * 8
    asm.place(fin.translate(x, 0, 0))

result = asm.to_object()
result.render("heat_sink.stl", resolution=64)
```

### .polyh

The `.polyh` language has no loops, so each fin must be placed individually. For many fins, the Python approach is more concise. For a small fixed number, `.polyh` is clear and readable:

```
# heat_sink.polyh

define base
  units mm
  cube
    width  60
    depth  40
    height  5
  end
end

define fin
  units mm
  cube
    width   2
    depth  40
    height 20
    move z 12.5
  end
end

assemble heat_sink
  units mm

  place base at origin

  place fin at (-24, 0, 0)
  place fin at (-16, 0, 0)
  place fin at ( -8, 0, 0)
  place fin at (  0, 0, 0)
  place fin at (  8, 0, 0)
  place fin at ( 16, 0, 0)
  place fin at ( 24, 0, 0)

end

export
  format stl
  quality high
  file   heat_sink.stl
end
```

Compile:

```bash
polyhedra -c heat_sink.polyh -o stl
```

---

## Reusing the Same define at Multiple Positions

The pipe fitting and heat sink both demonstrate the key benefit of the `define`/`assemble` split: **define a shape once, use it as many times as needed.**

In Python, the pattern is:

```python
# Define the shape once
m3_hole = ph.Object(ph.CYLINDER, 1.6, 20)

# Use it at N positions
positions = [(-25, -15, 0), (25, -15, 0), (-25, 15, 0), (25, 15, 0)]
for x, y, z in positions:
    asm.cut(m3_hole.translate(x, y, z))
```

In `.polyh`, the same `define` name appears in multiple `cut` or `place` lines with different `at` coordinates.

---

## Using intersect

`intersect` keeps only the volume shared between the assembly and the given object. It is useful for clipping a complex assembly to a bounding shape.

Python:

```python
# Clip the heat sink to a cylindrical envelope
clip = ph.Object(ph.CYLINDER, 28, 30)
clipped = asm.intersect(clip).to_object()
```

`.polyh`:

```
assemble clipped_sink
  place heat_sink at origin
  intersect clip  at origin
end
```

---

## Multi-file .polyh Projects

For larger projects, split each `define` into its own file and import them into a `main.polyh` using `use`:

```
# main.polyh
use pipe_body
use flange
use bore

assemble pipe_fitting
  place pipe_body at origin
  place flange    at (0, 0,  44)
  place flange    at (0, 0, -44)
  cut   bore      at origin
end
```

`use pipe_body` imports all `define` blocks from `pipe_body.polyh` in the same directory. See [Tutorial 5: Writing .polyh Files](05_polyh_language.md) for more on multi-file projects.

---

## Tips

- **Build parts at the origin, position in the assembly.** A `define` should describe the shape without worrying about where it will end up. The `assemble` block is where geometry gets positioned.
- **Name your defines clearly.** `m3_clearance_hole` is better than `hole1`. You will thank yourself when placing the same define at eight different positions.
- **Use Python for repetition.** The `.polyh` language has no loops or variables. For anything beyond a handful of repeated operations, Python's `for` loop is the right tool.
- **Validate before meshing.** `ph.validate("myfile.polyh")` checks syntax without running the SDF kernel — fast feedback when editing `.polyh` files.

---

## Next Steps

- [Tutorial 3: Manipulations](03_manipulations.md) — add fillets, chamfers, and shells to assembled parts
- [Tutorial 5: Writing .polyh Files](05_polyh_language.md) — full coverage of the `.polyh` DSL including multi-file projects
- [Tutorial 6: Python Workflow](06_python_workflow.md) — use Python loops and functions to generate families of parts
