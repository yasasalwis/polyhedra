# polyhedra Language Reference

The `.polyh` language is an indentation-friendly, plain-English DSL for describing 3D geometry. It compiles directly to STL, OBJ, PLY, or GLB via the Rust SDF kernel.

---

## File structure

A `.polyh` file is a sequence of top-level blocks:

```
use <filename>          # optional — import objects from another file
define <name>           # define a reusable object
  ...
end
assemble <name>         # combine objects into a final part
  ...
end
export                  # optional — set default output settings
  ...
end
```

**Rules:**
- Blocks are delimited by their keyword and `end`.
- Indentation is cosmetic — the parser does not care about indentation levels.
- `#` starts a comment to the end of the line.
- At least one `define` or `assemble` block is required to compile geometry.
- All dimensions default to **millimetres** unless a `units` statement is present.

---

## `use` — import a file

```
use hello
```

Imports all `define` blocks from `hello.polyh` (same directory). Imported names are available in `assemble` blocks.

---

## `define` — define an object

```
define bracket
  units mm

  cube
    width  60
    depth  40
    height  5
  end

  chamfer 1.5
end
```

A `define` block contains:
1. An optional `units` statement.
2. One or more primitive blocks.
3. Zero or more manipulation statements.

Multiple primitives inside a single `define` are automatically unioned together.

---

## Units

```
units mm    # millimetres (default)
units cm    # centimetres  (× 10)
units m     # metres       (× 1000)
units in    # inches       (× 25.4)
units ft    # feet         (× 304.8)
```

A `units` statement inside a `define` or `assemble` block applies to all dimensions within that block. Individual primitives can override with their own `units` statement.

---

## Primitives

Each primitive is opened by its keyword and closed by `end`.

### `cube`

An axis-aligned rectangular box centred at the origin.

```
cube
  width  60       # X dimension
  depth  40       # Y dimension
  height 20       # Z dimension
end
```

`depth` and `height` default to the value of `width` if omitted (making a true cube).

### `sphere`

A sphere centred at the origin.

```
sphere
  radius 10
end
```

### `cylinder`

A cylinder centred at the origin, axis along Z.

```
cylinder
  radius  5
  height 30
end
```

### `cone`

A cone or truncated cone (frustum) centred at the origin, axis along Z.

```
cone
  base_radius 8    # radius at the bottom (Z = -height/2)
  top_radius  0    # 0 = sharp tip; > 0 = flat-top frustum
  height     20
end
```

`base_radius` can also be written as `radius`. `top_radius` can also be written as `top`.

### `torus`

A torus (donut) in the XY plane.

```
torus
  major 20    # distance from centre to tube centre
  minor  4    # tube radius
end
```

`major` can also be written as `major_radius`. `minor` can also be written as `minor_radius`.

### `pyramid`

A square or rectangular pyramid, base in the XY plane, apex pointing in +Z.

```
pyramid
  base_width 10
  base_depth 10    # defaults to base_width if omitted
  height     15
end
```

`base_width` can also be written as `base`. `base_depth` defaults to `base_width`.

### `prism`

A right prism with a regular n-sided polygon cross-section, axis along Z.

```
prism
  sides       6    # number of sides (6 = hexagon)
  flat_to_flat 12  # diameter of the inscribed circle (wrench-flat size)
  height      20
end
```

`flat_to_flat` can also be written as `radius`.

---

## Move statements

Any primitive can be offset from its default centred position using `move`:

```
cube
  width  60
  depth   5
  height 40
  move z 22.5    # shift +22.5 mm along Z
end
```

Multiple `move` statements are applied in order:

```
cylinder
  radius 4
  height 20
  move x 10
  move z  5
end
```

Axes: `x`, `y`, `z`.

---

## Manipulations

Manipulations appear inside `define` or `assemble` blocks (after the primitive blocks) and transform the combined geometry.

### `chamfer <radius>`

Bevel all edges by `radius` mm.

```
define block
  cube
    width 20
  end
  chamfer 2
end
```

### `fillet <radius>`

Round all edges and corners by `radius` mm. Implemented as an exact SDF offset.

```
fillet 1.5
```

### `shell <thickness>`

Hollow the object, leaving a wall of `thickness` mm.

```
shell 2.0
```

The centre of the object becomes air; only the outer shell remains.

### `hole <diameter> [<depth>]`

Cut a cylindrical bore through the object. `depth` is optional.

```
hole 3.2         # M3 clearance hole, full depth
hole 3.2 10      # 10 mm deep
```

### `thread <diameter> [<pitch>]`

Mark a thread feature. Currently recorded in the AST; geometry modelled as a hole.

```
thread 3.0 0.5   # M3 × 0.5 thread
```

### `pattern <name> <count>`

Repeat a named feature. Currently recorded in the AST; geometry passed through unchanged (future: linear/circular arrays).

```
pattern m3_hole 4
```

---

## `assemble` — combine objects

```
assemble bracket_with_holes
  units mm

  place bracket  at origin
  cut   m3_hole  at (-25, -15, 0)
  cut   m3_hole  at ( 25, -15, 0)
  join  bracket2 at (0, 0, 30)

end
```

### Operations

| Keyword | Effect |
|---|---|
| `place <name> at <position>` | Union — add the named object |
| `join  <name> at <position>` | Alias for `place` |
| `cut   <name> at <position> [pointing <dir>]` | Difference — subtract the named object |
| `subtract <name> at <position>` | Alias for `cut` |
| `intersect <name> at <position>` | Keep only the overlapping volume |

### Positions

```
at origin           # place at (0, 0, 0)
at (x, y, z)        # place at explicit coordinates (in the block's unit scale)
```

### Pointing direction (for `cut`)

Optional — recorded in the AST but not yet used to orient the cut object.

```
cut m3_hole at (0, 0, 0) pointing up
```

Valid directions: `up`, `down`, `left`, `right`, `+x`, `-x`, `+y`, `-y`, `+z`, `-z`.

---

## `export` — default output settings

```
export
  format  stl       # stl | obj | ply | glb
  quality high      # low | medium | high | ultra
  file    output.stl
end
```

When compiling via the CLI, these settings are used as defaults and can be overridden by CLI flags. When compiling via `ph.compile_polyh()`, the `output` and `resolution` arguments take precedence.

---

## Full example

```polyhedra
# bracket.polyh
# An L-shaped mounting bracket with four M3 mounting holes.

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
    width 60
    depth  5
    height 40
    move z 22.5    # sit on top of the 5mm base
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

  chamfer 1.0
end

export
  format  stl
  quality high
  file    bracket.stl
end
```

Compile:
```bash
polyhedra -c bracket.polyh -o stl --quality high
```

---

## Grammar summary

```
file          = top_level_item*
top_level_item = use_stmt | define_block | assemble_block | export_block

use_stmt      = "use" ident

define_block  = "define" ident define_body_item* "end"
define_body_item = units_stmt | primitive_block | manipulation

primitive_block = prim_kw prim_body_item* "end"
prim_kw       = "cube" | "sphere" | "cylinder" | "cone"
              | "torus" | "pyramid" | "prism"
prim_body_item = units_stmt | prop_stmt | move_stmt

prop_stmt     = ident number
move_stmt     = "move" axis number
units_stmt    = "units" unit_kw
unit_kw       = "mm" | "cm" | "m" | "in" | "ft"
axis          = "x" | "y" | "z"

manipulation  = chamfer_stmt | fillet_stmt | shell_stmt
              | hole_stmt | thread_stmt | pattern_stmt
chamfer_stmt  = "chamfer" number
fillet_stmt   = "fillet"  number
shell_stmt    = "shell"   number
hole_stmt     = "hole"    number number?
thread_stmt   = "thread"  number number?
pattern_stmt  = "pattern" ident number

assemble_block = "assemble" ident assemble_body_item* "end"
assemble_body_item = units_stmt | assemble_op | manipulation

assemble_op   = place_stmt | cut_stmt | join_stmt
              | intersect_stmt | subtract_stmt
place_stmt    = "place" ident "at" position
join_stmt     = "join"  ident "at" position
cut_stmt      = "cut"   ident "at" position ("pointing" direction)?
intersect_stmt= "intersect" ident "at" position
subtract_stmt = "subtract"  ident "at" position

position      = "origin" | "(" number "," number "," number ")"
direction     = "up"|"down"|"left"|"right"|"+x"|"-x"|"+y"|"-y"|"+z"|"-z"

export_block  = "export" export_body_item* "end"
export_body_item = export_format_stmt | export_quality_stmt | export_file_stmt

number        = [-]?[0-9]+("."[0-9]+)?
ident         = [a-zA-Z_][a-zA-Z0-9_]*
```
