# Tutorial 5 — Writing .polyh Files

The `.polyh` language is a plain-English DSL for describing 3D geometry. It compiles directly to STL, OBJ, PLY, or GLB via the same Rust SDF kernel used by the Python API. This tutorial is a hands-on introduction to the language: its structure, all seven primitives, manipulations, assemblies, and multi-file projects.

Topics covered:

- Why `.polyh` exists
- File structure: `use` → `define` → `assemble` → `export`
- Writing your first `define` block
- All 7 primitives with examples
- The `move` statement for positioning within a define
- Manipulations in define blocks
- Writing an `assemble` block
- The `export` block
- Multi-file projects with `use`
- Common mistakes and how to fix them

---

## Why .polyh Exists

Most 3D geometry formats are binary blobs or XML. They are difficult to read, generate, and diff. The `.polyh` language exists so that geometry can be described in a form that:

- A human engineer can write and review without any GUI
- A version-control system (git) can diff meaningfully
- An LLM can generate or modify as part of a larger workflow
- The Rust compiler can turn directly into a production-quality mesh

If you are working interactively and need Python logic (loops, conditionals, variables), use the Python API. If you want a self-contained, human-readable geometry file, write a `.polyh` file.

---

## File Structure

A `.polyh` file is a sequence of top-level blocks. The order within the file matters only for `use` statements, which must appear before the names they import are used.

```
use <filename>          # optional — import defines from another file
define <name>           # define a reusable shape
  ...
end
assemble <name>         # combine shapes into a final part
  ...
end
export                  # optional — set default output settings
  ...
end
```

**Rules:**

- Blocks are delimited by their keyword and `end`.
- Indentation is cosmetic — the parser does not enforce indentation levels.
- `#` starts a comment to the end of the line.
- At least one `define` or `assemble` block is required.
- All dimensions default to millimetres unless a `units` statement is present.
- The language has no variables, no arithmetic, and no conditionals. All values are literal numbers.

---

## Writing Your First define Block

A `define` block gives a name to a shape. Here is the simplest possible example:

```
define puck
  units mm
  cylinder
    radius 20
    height  8
  end
end
```

To compile this with the CLI:

```bash
polyhedra -c puck.polyh -o stl
```

This renders the first `define` or `assemble` block it finds. For a single-define file, you get a cylinder.

A `define` block can contain:

1. An optional `units` statement
2. One or more primitive blocks
3. Zero or more manipulation statements

When a `define` contains multiple primitives, they are automatically unioned together:

```
define bracket
  units mm

  # Base plate
  cube
    width  60
    depth  40
    height  5
  end

  # Vertical wall (unioned with base plate automatically)
  cube
    width  60
    depth   5
    height 40
    move z 22.5
  end
end
```

---

## Units

A `units` statement inside a `define` or `assemble` block sets the measurement unit for all dimensions in that block:

```
units mm    # millimetres (default)
units cm    # centimetres (× 10)
units m     # metres (× 1000)
units in    # inches (× 25.4)
units ft    # feet (× 304.8)
```

Without a `units` statement, millimetres are assumed.

---

## All 7 Primitives

### cube

An axis-aligned rectangular box centred at the origin.

```
cube
  width  60     # X dimension
  depth  40     # Y dimension
  height 20     # Z dimension
end
```

If `depth` and `height` are omitted, they default to `width` (making a true cube).

### sphere

A sphere centred at the origin.

```
sphere
  radius 10
end
```

### cylinder

A cylinder centred at the origin with its axis along Z.

```
cylinder
  radius  8
  height 30
end
```

### cone

A cone or truncated cone (frustum) centred at the origin, axis along Z.

```
cone
  base_radius 15    # radius at the bottom (Z = -height/2)
  top_radius   0    # 0 = sharp tip; > 0 = flat-top frustum
  height      25
end
```

`base_radius` can also be written as `radius`. `top_radius` can also be written as `top`.

A frustum (flat-top cone):

```
cone
  base_radius 15
  top_radius   8
  height      20
end
```

### torus

A torus (donut shape) in the XY plane.

```
torus
  major 25    # distance from the torus centre to the tube centre
  minor  5    # tube radius
end
```

`major` can also be written as `major_radius`. `minor` can also be written as `minor_radius`.

### pyramid

A rectangular pyramid with its base in the XY plane and apex pointing in +Z.

```
pyramid
  base_width 20
  base_depth 20    # defaults to base_width if omitted
  height     15
end
```

`base_width` can also be written as `base`.

### prism

A right prism with a regular n-sided polygon cross-section, axis along Z.

```
prism
  sides        6      # 6 = hexagon, 3 = triangle, 8 = octagon
  flat_to_flat 11.0   # inscribed circle diameter (wrench-flat size)
  height        5.5
end
```

`flat_to_flat` can also be written as `radius`.

---

## The move Statement

By default, every primitive is centred at the origin. Use `move` inside a primitive block to offset it along an axis:

```
cube
  width  60
  depth   5
  height 40
  move z 22.5     # shift +22.5 mm along Z
end
```

Multiple `move` statements stack in order:

```
cylinder
  radius 5
  height 20
  move x 15
  move z 10
end
```

Valid axes: `x`, `y`, `z`. The value is in the block's unit scale.

### Practical tip: placing on top of another shape

To place a shape on top of another, the `move z` value is `(height_of_lower_shape / 2) + (height_of_upper_shape / 2)`:

- Base plate: 5 mm tall → top face at Z = +2.5
- Wall: 40 mm tall → centre at Z = +2.5 + 20 = +22.5
- Therefore: `move z 22.5`

---

## Manipulations in define Blocks

Manipulations appear after all primitive blocks inside a `define` and transform the combined geometry:

```
define rounded_block
  units mm

  cube
    width  40
    depth  30
    height 20
  end

  fillet 2.0     # round all edges
end
```

Available manipulations:

| Statement | Effect |
|---|---|
| `fillet <r>` | Round all edges and corners by radius `r` |
| `chamfer <r>` | Bevel all edges by `r` (45° flat cut) |
| `shell <t>` | Hollow out, leaving wall thickness `t` |
| `hole <d> [<depth>]` | Cut a cylindrical bore of diameter `d` |
| `thread <d> [<pitch>]` | Mark a threaded hole (modelled as a bore) |
| `pattern <name> <n>` | Record a repeated feature (future: arrays) |

Multiple manipulations are applied in order:

```
define processed
  units mm
  cube
    width 30
    depth 30
    height 15
  end
  chamfer 1.0
  shell   2.0
end
```

---

## Writing an assemble Block

An `assemble` block composes previously-defined shapes into a final part using boolean operations.

```
assemble bracket_with_holes
  units mm

  place base_plate    at origin
  place vertical_wall at origin

  cut m3_hole at (-25, -15, 0)
  cut m3_hole at ( 25, -15, 0)
  cut m3_hole at (-25,  15, 0)
  cut m3_hole at ( 25,  15, 0)

  fillet 1.5
end
```

### Operations

| Keyword | Effect |
|---|---|
| `place <name> at <position>` | Union — add the named shape |
| `join <name> at <position>` | Alias for `place` |
| `cut <name> at <position>` | Difference — subtract the named shape |
| `subtract <name> at <position>` | Alias for `cut` |
| `intersect <name> at <position>` | Keep only the overlapping volume |

### Positions

```
at origin           # equivalent to (0, 0, 0)
at (x, y, z)        # explicit coordinates in the block's unit scale
```

### Pointing direction (for cut)

Optional — recorded for future directional-cut support:

```
cut m3_hole at (0, 10, 0) pointing up
```

Valid directions: `up`, `down`, `left`, `right`, `+x`, `-x`, `+y`, `-y`, `+z`, `-z`.

---

## The export Block

The `export` block sets default output settings. When compiling via the CLI, these are used as defaults and can be overridden by flags.

```
export
  format  stl       # stl | obj | ply | glb
  quality high      # low | medium | high | ultra
  file    part.stl  # output file name
end
```

Quality levels map to mesh resolution:

| Quality | Approx. resolution |
|---|---|
| `low` | 16 voxels/axis |
| `medium` | 32 voxels/axis |
| `high` | 64 voxels/axis |
| `ultra` | 128 voxels/axis |

---

## A Complete .polyh File

Putting it all together: the L-bracket from [Tutorial 1](01_first_part.md) as a complete, self-contained `.polyh` file.

```
# bracket.polyh
# L-shaped mounting bracket with four M3 clearance holes.

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

```bash
polyhedra -c bracket.polyh -o stl --quality high
```

---

## Multi-file Projects with use

For larger projects, split each `define` into its own file and import them:

```
# main.polyh
use base_plate
use vertical_wall
use m3_hole

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
```

`use base_plate` imports all `define` blocks from `base_plate.polyh` in the same directory. The `.polyh` extension is implicit.

A typical multi-file project layout:

```
my_part/
  main.polyh          ← assemble + export
  base_plate.polyh    ← define base_plate
  vertical_wall.polyh ← define vertical_wall
  m3_hole.polyh       ← define m3_hole
```

`use` statements must appear before the names they introduce are used in `assemble` blocks.

---

## Validating a .polyh File

Before meshing, check syntax:

```bash
polyhedra --validate bracket.polyh
```

Or from Python:

```python
import polyhedra as ph
errors = ph.validate("bracket.polyh")
for msg in errors:
    print(msg)
```

An empty list means the file is syntactically valid.

---

## Common Mistakes and How to Fix Them

### Forgetting `end`

Every primitive and block needs a closing `end`:

```
# Wrong
define puck
  cylinder
    radius 20
    height  8
  # missing end for cylinder
end

# Correct
define puck
  cylinder
    radius 20
    height  8
  end
end
```

### Using variables or arithmetic

The `.polyh` language has no variables and no arithmetic. All values must be literal numbers.

```
# Wrong — no variables allowed
define ring
  torus
    major radius * 2    # not valid
    minor 4
  end
end

# Correct — compute the value yourself
define ring
  torus
    major 40
    minor  4
  end
end
```

If you need parametric geometry, use the Python API (see [Tutorial 6](06_python_workflow.md)).

### Placing a name that was not defined

All names used in `place`, `cut`, `join`, `intersect`, and `subtract` must be defined in a `define` block (or imported with `use`) before the `assemble` block that uses them.

```
# Wrong — bracket_wall is not defined
assemble thing
  place bracket_wall at origin
end

# Correct
define bracket_wall
  ...
end
assemble thing
  place bracket_wall at origin
end
```

### Expecting the assemble block to produce a file without an export block

Running `polyhedra -c myfile.polyh` without an `export` block will still work — the CLI uses the `-o` flag to set the format. But if you want the file name and quality to be embedded in the `.polyh` file, add an `export` block.

### Move values in wrong units

`move` values are in the block's unit scale. If you write `units mm` and then `move z 5`, the shift is 5 mm. If you forgot `units` and the default is already mm, this is fine. If you wrote `units cm`, `move z 5` shifts 5 cm = 50 mm.

---

## Next Steps

- [Tutorial 2: Assemblies](02_assemblies.md) — deeper coverage of the `assemble` block and multi-part projects
- [Tutorial 6: Python Workflow](06_python_workflow.md) — when the `.polyh` language's limits push you to Python
- [Language Reference](../language-reference.md) — the full grammar, all keywords, and all options
