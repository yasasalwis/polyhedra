# Tutorial 4 — Export Formats: STL, OBJ, PLY, GLB

Polyhedra can export geometry to four mesh formats. This tutorial explains what each format is, when to use it, how to control mesh quality, and what file sizes and triangle counts to expect.

Topics covered:

- What each format is and when to reach for it
- Resolution and quality settings
- CLI export flags
- Python export API
- File size and triangle count expectations

---

## The Four Formats

### STL — Stereolithography

STL is the lingua franca of 3D printing and CNC machining. It stores a triangle mesh as a flat list of facets with normals — nothing else. No colour, no material, no scene hierarchy.

**Use STL when:**

- Sending a part to a slicer (PrusaSlicer, Cura, Bambu Studio)
- Sending toolpaths to CAM software (Fusion 360, FreeCAD)
- Opening in MeshLab, Meshmixer, or Windows 3D Builder
- Sharing with anyone who just needs the shape

**Limitations:** Binary STL is compact but carries no metadata. It cannot represent colour, UV coordinates, or named parts.

### OBJ — Wavefront Object

OBJ is a human-readable text format that includes vertex positions, face indices, and vertex normals. Because it is plain text it is easy to inspect and diff. Most 3D software can import OBJ.

**Use OBJ when:**

- You need a format a human can open in a text editor and read
- You are importing into Blender, Maya, or Cinema 4D for rendering
- You want vertex normals preserved for smooth-shading in a renderer
- You are handing geometry off to another team and want traceability

**Limitations:** OBJ files are significantly larger than binary STL or GLB for the same mesh. No animation, no PBR materials.

### PLY — Polygon File Format

PLY (also called Stanford Triangle Format) is commonly used in 3D scanning, photogrammetry, and point-cloud workflows. It supports arbitrary per-vertex and per-face attributes (colour, confidence, curvature, etc.).

**Use PLY when:**

- You are feeding geometry into a scanning or point-cloud pipeline
- You are using MeshLab for analysis or repair
- You need to attach per-vertex colour or custom attributes to the mesh

**Limitations:** PLY is less universally supported than STL or OBJ for printing workflows. Most slicers cannot read it.

### GLB — GL Transmission Format (Binary)

GLB is the binary form of glTF, the "JPEG of 3D." It is the native format for web-based 3D (Three.js, Babylon.js, model-viewer), game engines (Godot, Bevy), and AR/VR platforms (Quest, Vision Pro). GLB bundles geometry, materials, and scene hierarchy into a single compact binary.

**Use GLB when:**

- Embedding a 3D model in a web page with Three.js or `<model-viewer>`
- Importing into Godot, Unity, or Unreal Engine
- Previewing in AR on iOS (Quick Look) or Android (Scene Viewer)
- Sharing a model for interactive viewing (Sketchfab, Spline, etc.)

**Limitations:** GLB is not accepted by most slicers. It carries more overhead than STL for pure-shape use cases.

---

## Resolution and Quality Settings

Polyhedra meshes geometry by evaluating the SDF on a 3D grid (Dual Contouring). The grid resolution directly controls triangle count, detail level, and render time.

### resolution= (voxels per axis)

```python
obj.render("out.stl", resolution=16)    # fast preview
obj.render("out.stl", resolution=32)    # general use
obj.render("out.stl", resolution=64)    # production
obj.render("out.stl", resolution=128)   # highly detailed
```

| `resolution` | CLI `--quality` | Typical triangles | Typical render time | Use case |
|---|---|---|---|---|
| 16 | `--quality low` | 200–2 000 | < 0.1 s | Quick preview during iteration |
| 32 | `--quality medium` | 1 000–15 000 | 0.1–0.5 s | General development |
| 64 | `--quality high` | 5 000–80 000 | 0.5–3 s | Final parts for printing or web |
| 128 | `--quality ultra` | 20 000–400 000 | 3–30 s | Highly detailed or large models |

Triangle counts vary widely depending on geometry complexity. A simple cube has fewer triangles than a torus with a fillet.

### refinement= (voxel size in mm)

Instead of specifying voxels per axis, you can specify the physical voxel size in millimetres. This is more predictable for parts with known dimensions:

```python
obj.render("out.stl", refinement=0.5)    # 0.5 mm voxel — good for most printed parts
obj.render("out.stl", refinement=0.2)    # 0.2 mm voxel — fine detail
obj.render("out.stl", refinement=2.0)    # 2.0 mm voxel — fast preview
```

`refinement` overrides `resolution` if both are supplied. Choose `refinement` when you care about the physical accuracy of the mesh relative to a known part size.

### Practical guidance

- **During design:** use `resolution=16` or `refinement=2.0`. Renders complete in under a second. Good enough to see the overall shape.
- **For slicing:** use `resolution=64` or `refinement=0.4`. Matches typical 3D printer layer/feature resolution.
- **For web/AR:** use `resolution=64`. GLB files stay under a few hundred KB, which loads fast in a browser.
- **For highly curved surfaces** (fillets, tori): increase resolution or decrease refinement. Curved edges are the first features to look faceted at low resolution.

---

## CLI Export

### Single format

```bash
polyhedra -c part.polyh -o stl
polyhedra -c part.polyh -o glb
polyhedra -c part.polyh -o obj
polyhedra -c part.polyh -o ply
```

### Multiple formats in one pass

```bash
polyhedra -c part.polyh -o stl,glb,obj
```

### All supported formats

```bash
polyhedra -c part.polyh -o all
```

### Quality

```bash
polyhedra -c part.polyh -o stl --quality low     # resolution=16
polyhedra -c part.polyh -o stl --quality medium  # resolution=32
polyhedra -c part.polyh -o stl --quality high    # resolution=64
polyhedra -c part.polyh -o stl --quality ultra   # resolution=128
```

### Output file name

The default output file name is derived from the `assemble` block name and the format extension. Override it with the `export` block in your `.polyh` file:

```
export
  format stl
  quality high
  file   my_part.stl
end
```

---

## Python Export API

### .render(path)

Mesh and write to a file. The format is inferred from the extension:

```python
import polyhedra as ph

obj = ph.Object(ph.CUBE, 40, 40, 40).fillet(2.0)

obj.render("part.stl")                       # STL, resolution=32 (default)
obj.render("part.glb", resolution=64)        # GLB, high quality
obj.render("part.obj", refinement=0.5)       # OBJ, 0.5 mm voxels
obj.render("part.ply", resolution=32)        # PLY
```

`.render()` returns `self`, enabling chained exports:

```python
(obj
    .render("part.stl", resolution=64)
    .render("part.glb", resolution=64)
    .render("part_preview.stl", resolution=16))
```

### .to_bytes(fmt)

Mesh and return the file content as `bytes`. Useful when you want to write to a custom path, upload to an API, or process the bytes before saving:

```python
stl_bytes = obj.to_bytes("stl", resolution=64)
with open("output/bracket_v2.stl", "wb") as f:
    f.write(stl_bytes)

glb_bytes = obj.to_bytes("glb", resolution=64)
# upload to a CDN, pass to a web framework, etc.
```

### .mesh()

Returns the raw mesh object (vertices + faces) without writing a file. Use this when you want to inspect or post-process the mesh:

```python
m = obj.mesh(resolution=64)
# m.vertices: array of (x, y, z) float
# m.faces: array of (i, j, k) int indices
print(f"Vertices: {len(m.vertices)}, Faces: {len(m.faces)}")
```

### ph.compile_polyh()

Compile a `.polyh` file's `assemble` block and return bytes:

```python
import polyhedra as ph

data = ph.compile_polyh("bracket.polyh", output="glb", resolution=64)
with open("bracket.glb", "wb") as f:
    f.write(data)
```

---

## File Size Expectations

These are approximate ranges for a single mechanical part similar to the L-bracket from [Tutorial 1](01_first_part.md):

| Format | resolution=16 | resolution=32 | resolution=64 | resolution=128 |
|---|---|---|---|---|
| STL (binary) | 5–30 KB | 30–150 KB | 150–800 KB | 0.8–5 MB |
| OBJ (text) | 15–90 KB | 80–450 KB | 400–2 MB | 2–15 MB |
| PLY (binary) | 5–25 KB | 25–120 KB | 120–700 KB | 0.7–4 MB |
| GLB (binary) | 4–20 KB | 20–100 KB | 100–500 KB | 0.5–3 MB |

GLB is typically the most compact binary format because it uses a more efficient index buffer. OBJ is the largest because it is plain text.

For web delivery (Three.js, Babylon.js), target GLB at `resolution=64`: typically 100–500 KB, which loads in under a second on a typical connection.

For 3D printing, STL at `resolution=64` is more than sufficient for all consumer and professional FDM/SLA printers. The printer never uses more detail than its own layer height (usually 0.1–0.2 mm) regardless of how many triangles are in the file.

---

## Choosing Format and Resolution: Decision Guide

```
Need to 3D print it?
  → STL, resolution=64

Need to view it on a web page or in a game engine?
  → GLB, resolution=64

Need to open it in Blender/Maya for rendering?
  → OBJ, resolution=64 (vertex normals make it shade smoothly)

Need to analyse it in MeshLab or a scanning pipeline?
  → PLY, resolution=64

Just checking the shape during development?
  → Any format, resolution=16
```

---

## Next Steps

- [Tutorial 5: Writing .polyh Files](05_polyh_language.md) — use the `export` block to set default format and quality in your `.polyh` file
- [Tutorial 6: Python Workflow](06_python_workflow.md) — batch-export a family of parts to multiple formats in one script
- [Quick Start](../quick-start.md) — go back to the beginning if you need a refresher
