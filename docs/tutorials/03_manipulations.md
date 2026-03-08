# Tutorial 3 — Manipulations: Fillets, Shells, and More

Manipulations transform the combined geometry of a part after all primitives and boolean operations have been applied. Polyhedra implements manipulations at the SDF level, which means they are geometrically exact — no mesh post-processing, no artefacts at high curvature.

This tutorial covers every manipulation with a before/after example in both Python and `.polyh`, and gives guidance on when to choose one over another.

Topics covered:

- `fillet(r)` — round all edges
- `chamfer(r)` — bevel all edges
- `shell(t)` — hollow out the part
- `offset(a)` — grow or shrink uniformly
- `smooth_union(b, blend)` — blend two objects together
- When to use which manipulation
- Chaining manipulations

---

## fillet(r) — Round All Edges

A fillet replaces each sharp edge and corner with a smooth arc of radius `r`. In polyhedra, fillets are implemented as an SDF inward/outward offset followed by a restoration, so all edges — including concave inside corners — are rounded simultaneously and correctly.

### Python

```python
import polyhedra as ph

# Before: sharp-edged box
sharp = ph.Object(ph.CUBE, 40, 30, 20)

# After: all edges rounded to 2 mm radius
rounded = ph.Object(ph.CUBE, 40, 30, 20).fillet(2.0)

sharp.render("fillet_before.stl",   resolution=32)
rounded.render("fillet_after.stl",  resolution=32)
```

Fillet can also be applied after boolean operations:

```python
hole  = ph.Object(ph.CYLINDER, 3, 25)
block = ph.Object(ph.CUBE, 30, 30, 20).difference(hole)
part  = block.fillet(1.5)
part.render("filleted_part.stl", resolution=64)
```

### .polyh

```
define filleted_block
  units mm

  cube
    width  40
    depth  30
    height 20
  end

  fillet 2.0
end
```

In `.polyh`, `fillet` is a manipulation statement inside a `define` or `assemble` block. It always applies to the entire combined geometry of that block.

### Choosing a fillet radius

- The fillet radius must be smaller than half the thinnest wall in the part. A 1.5 mm fillet on a 2 mm wall will eat the wall away.
- Start small (0.5–1 mm) for functional parts; use larger values (2–4 mm) for consumer products.
- For 3D printing, a minimum fillet of 0.4 mm keeps detail within printer resolution.

---

## chamfer(r) — Bevel All Edges

A chamfer cuts a flat 45-degree bevel at every edge. The result looks more machined and industrial than a fillet.

### Python

```python
import polyhedra as ph

# A hex nut body with chamfered top and bottom edges
nut = ph.Object(ph.PRISM, 6, 11.0, 5.5)   # M6 hex nut geometry
chamfered_nut = nut.chamfer(0.5)

chamfered_nut.render("chamfered_nut.stl", resolution=64)
```

### .polyh

```
define chamfered_block
  units mm

  cube
    width  40
    depth  30
    height 20
  end

  chamfer 1.5
end
```

### fillet vs chamfer

| | Fillet | Chamfer |
|---|---|---|
| Shape | Smooth arc | Flat 45° cut |
| Use case | Consumer parts, organic forms, ergonomics | Machined parts, chamfer-for-printability |
| 3D printing | Reduces stress concentrations | Eliminates elephant foot |
| Computation | Slightly more expensive | Fast |

When in doubt, use `fillet` for printed plastic and `chamfer` for metal-look parts.

---

## shell(t) — Hollow Out

`shell(t)` turns a solid object into a thin-walled hollow shell with wall thickness `t`. The interior becomes empty space; the exterior boundary is unchanged.

This is useful for:

- Reducing material use in 3D printing (when no infill is needed)
- Enclosures, boxes, and housings
- Lightweight structural parts

### Python

```python
import polyhedra as ph

# A solid cube, then hollowed to 2 mm walls
solid = ph.Object(ph.CUBE, 50, 50, 50)
hollow = solid.shell(2.0)

solid.render("solid_cube.stl",  resolution=32)
hollow.render("hollow_cube.stl", resolution=32)

# Verify: the centre should now be outside the solid (empty)
assert hollow.distance(0, 0, 0) > 0.0
# Verify: a point just inside the outer wall should be inside
assert hollow.distance(24.5, 0, 0) < 0.0
```

### .polyh

```
define enclosure
  units mm

  cube
    width  80
    depth  60
    height 40
  end

  shell 2.5
end
```

### shell with a boolean opening

Shell creates a completely closed hollow object. To add an opening (such as the top face of a box lid), subtract a slightly oversized box from the top after shelling:

```python
import polyhedra as ph

box   = ph.Object(ph.CUBE, 80, 60, 40).shell(2.5)
lid   = ph.Object(ph.CUBE, 82, 62, 22)            # wider and taller than the top half
open_box = box.difference(lid.translate(0, 0, 21)) # remove the top
open_box.render("open_box.stl", resolution=64)
```

---

## offset(a) — Grow or Shrink Uniformly

`offset(a)` expands or contracts the object by `a` mm in every direction simultaneously.

- Positive `a`: grows the object (like inflating a balloon)
- Negative `a`: shrinks the object (like deflating)

A small positive offset also rounds convex edges; a small negative offset trims them slightly.

### Python

```python
import polyhedra as ph

part    = ph.Object(ph.CUBE, 20, 20, 20)
grown   = part.offset(1.0)    # 22 × 22 × 22 mm
shrunk  = part.offset(-2.0)   # 16 × 16 × 16 mm

grown.render("offset_grown.stl",  resolution=32)
shrunk.render("offset_shrunk.stl", resolution=32)
```

### Common uses

- **Clearance fits:** shrink a peg by 0.1–0.2 mm so it slides into a hole.
- **Interference fits:** grow a peg by 0.05–0.1 mm for a press fit.
- **Minkowski rounding:** grow by a small positive value to round all convex edges without the cost of a full fillet.

### .polyh

`offset` is not a built-in manipulation statement in `.polyh` (use the Python API for offset operations).

---

## smooth_union(b, blend) — Blended Joins

`smooth_union` combines two objects like a regular union, but adds a smooth rounded fillet where their surfaces meet. The `blend` parameter controls the radius of the blending region.

This produces organic, continuous surfaces — think of two clay balls pressed together.

### Python

```python
import polyhedra as ph

# Two spheres that overlap slightly
sphere_a = ph.Object(ph.SPHERE, 10)
sphere_b = ph.Object(ph.SPHERE,  8).translate(14, 0, 0)

# Regular union: sharp crease at the join
joined   = sphere_a.union(sphere_b)

# Smooth union: continuous blend with a 5 mm radius
blended  = sphere_a.smooth_union(sphere_b, blend=5.0)

joined.render("union_sharp.stl",    resolution=64)
blended.render("union_smooth.stl",  resolution=64)
```

Increase `blend` for a softer, more gradual transition; decrease it for a tighter, more localised blend.

### When to use smooth_union

| Situation | Recommendation |
|---|---|
| Mechanical parts (brackets, enclosures) | `.union()` or `ph.Assembly.place()` |
| Organic shapes (handles, grips, sculptures) | `.smooth_union(blend=3.0–8.0)` |
| Two objects that should visually flow together | `.smooth_union()` |
| Logo or embossed text on a flat surface | `.smooth_union(blend=0.5–1.0)` |

### .polyh

`smooth_union` is not a top-level manipulation in `.polyh`. Use the Python API when you need blended joins.

---

## Chaining Manipulations

Manipulations can be chained. They apply in left-to-right order on the object:

```python
import polyhedra as ph

part = (
    ph.Object(ph.CUBE, 40, 30, 10)
    .fillet(1.5)         # round edges first
    .shell(1.2)          # then hollow it out
)
part.render("chained.stl", resolution=64)
```

In `.polyh`, list manipulations after the primitives in order:

```
define processed_block
  units mm

  cube
    width  40
    depth  30
    height 10
  end

  fillet 1.5
  shell  1.2
end
```

> **Order matters for shell.** Apply `fillet` or `chamfer` before `shell`. Filleting after shelling rounds both the inner and outer walls, which can look odd on thin shells.

---

## Manipulations in Assemblies

Manipulations can be applied to an entire assembly after all boolean operations are done:

```python
import polyhedra as ph

asm = (
    ph.Assembly("bracket")
    .place(ph.Object(ph.CUBE, 60, 40, 5))
    .place(ph.Object(ph.CUBE, 60, 5, 40).translate(0, 17.5, 22.5))
    .cut(ph.Object(ph.CYLINDER, 1.6, 20).translate(-25, -15, 0))
)

# Apply fillet to the entire assembled result
result = asm.to_object().fillet(1.5)
result.render("bracket_filleted.stl", resolution=64)
```

In `.polyh`, add the manipulation statement inside the `assemble` block after all `place`/`cut` lines:

```
assemble bracket
  units mm
  place base_plate    at origin
  place vertical_wall at origin
  cut   m3_hole       at (-25, -15, 0)
  fillet 1.5
end
```

---

## Quick Reference

| Manipulation | Python | .polyh |
|---|---|---|
| Round edges | `.fillet(r)` | `fillet r` |
| Bevel edges | `.chamfer(r)` | `chamfer r` |
| Hollow out | `.shell(t)` | `shell t` |
| Grow/shrink | `.offset(a)` | Python only |
| Blended join | `.smooth_union(b, blend=n)` | Python only |

---

## Next Steps

- [Tutorial 4: Export Formats](04_export_formats.md) — choose the right format for your workflow and configure quality settings
- [Tutorial 1: Your First Part](01_first_part.md) — if you skipped it, go back and build the L-bracket end to end
- [Tutorial 6: Python Workflow](06_python_workflow.md) — use Python variables to drive manipulation parameters parametrically
