# polyhedra — Rust kernel (`_core`)

This crate contains the entire geometry engine. It is published as a Python extension module (`polyhedra._core`) via PyO3/maturin, and also as a standalone CLI binary (`polyhedra`).

## Crate layout

```
src/
├── lib.rs              Public re-exports + PyO3 module registration
├── error.rs            PolyhedraError type
├── units.rs            Unit enum (mm/cm/m/in/ft) + scale factors
├── sdf/
│   ├── mod.rs          Sdf trait + SdfNode type alias
│   ├── primitives/     CubeSdf, SphereSdf, CylinderSdf, ConeSdf,
│   │                   TorusSdf, PyramidSdf, PrismSdf
│   ├── operations.rs   UnionNode, DifferenceNode, IntersectionNode,
│   │                   SmoothUnionNode, ChamferUnionNode
│   └── transform.rs    Translate, Scale, Rotate, Mirror
├── manipulations/
│   ├── morph.rs        OffsetNode, ShellNode, ElongateNode
│   └── pattern.rs      MirrorNode, RepeatNode, RepeatCircularNode
├── sketch/
│   ├── primitives.rs   2D SDFs: Circle, Rect, Polygon, Capsule, RoundedRect
│   └── ops.rs          Extrude, Revolve
├── mesher/
│   ├── mod.rs          Dual Contouring entry point (mesh + MeshConfig)
│   ├── dc.rs           Octree traversal + edge/vertex generation
│   └── qef.rs          Quadric Error Function solver (3×3 linear system)
├── export/
│   ├── mod.rs          ExportFormat enum + to_bytes/to_file dispatch
│   ├── stl.rs          Binary STL writer
│   ├── obj.rs          ASCII OBJ + MTL writer
│   ├── ply.rs          Binary PLY writer
│   └── gltf.rs         GLB (binary GLTF 2.0) writer
├── parser/
│   ├── grammar.pest    PEG grammar for the .polyh DSL
│   ├── ast.rs          All AST types
│   ├── mod.rs          pest parser → AST builder
│   └── eval.rs         AST → SDF node evaluator
├── python.rs           PyO3 bindings (feature = "python")
└── bin/
    └── polyhedra.rs    CLI entry point
```

## Building

```bash
# Library + CLI (no Python)
cargo build --release

# Library + Python extension (type-check only)
cargo check --features python

# Python extension (via maturin — from project root)
python -m maturin develop --features python

# Run all tests
cargo test
```

## Key design decisions

**SDF-first geometry** — every shape is `f(p: Vec3) -> f32`. Boolean ops are min/max. Fillets are smooth_min. No mesh operations until the very last step.

**Dual Contouring mesher** — produces feature-preserving meshes with sharp edges where the SDF has them. Uses a QEF solver per cell to place vertices on the isosurface.

**`SdfNode = Box<dyn Sdf>`** — the core type alias. The tree of SDF nodes is heap-allocated and dynamic-dispatched. `rayon` parallelises the grid evaluation across all voxels.

**`ArcSdf` for PyO3** — `#[pyclass]` cannot hold `Box<dyn Trait>` because it needs `Clone`. The `ArcSdf(Arc<dyn Sdf + Send + Sync>)` wrapper delegates `Sdf` calls and allows cheap `Arc::clone` when building compound nodes from Python.
