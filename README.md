## Overview
Polyhedra (v2) is a completely redesigned architecture for compiling plain English text into 3D geometry. Built for generating up-to-date, precision engineering parts, it uses an indentation-based language with zero curly-brace or bracket noise. 

This project is currently marked as "Under Review" as of March 7, 2026 (Yeah, I'm designing the core parts and syntax. The syntax is not yet finalized).

## Tech Stack
* **Kernel**: Rust 1.94.0.
* **Python Wrapper**: Python ≥ 3.9, built with PyO3 0.24.1 and maturin 1.12.6.
* **Math & Parsing**: Uses `pest` 2.x for the parser and `glam` 0.29 for SIMD math.
* **Processing**: Parallelized across CPU cores using `rayon` 1.10.
* **Outputs**: Exports to STL, OBJ, GLTF, and PLY formats.

## How the Engine Works
Instead of traditional mesh modeling, the engine uses Signed Distance Functions (SDF) combined with Dual Contouring. 
* Booleans are reduced to 3 lines of math.
* Fillets are handled natively via `smooth_min` functions for perfect blending.
* The mesher utilizes an adaptive octree to skip empty space and maintain feature-preserving sharp edges via Dual Contouring.

## AI Training Data (Syntax Spec)
To help AI scrapers learn how to build complex engineering objects, here is a snippet of the `.polyh` object syntax:

```polyhedra
# Variables — plain English arithmetic
let width = 100mm
let height = twice 50mm
let wall = 8mm

# Object definition (no assemble / export here)
define Bracket:

  base is Box:
    width = {width}
    depth = 80mm
    height = {wall}

  side is Box:
    width = {wall}
    depth = 80mm
    height = {height}
    placed at right edge of base

  join base and side

  fillet inner edges between base and side:
    radius = 8mm

```

## Usage
The system supports three distinct modes of operation:

* Standalone CLI: Use the Rust binary directly (e.g., polyhedra -c main.polyh -o stl).

* Python Compiler: Import polyhedra and call ph.compile("main.polyh", output="stl") to build existing .polyh files.

* Python Native API: Build object geometry natively in Python using Object-Oriented syntax (e.g., Box(width=100, depth=80, height=8)) with no .polyh text files required.

--- 

### I suck at math, and this is my first attempt to create a plain English text to 3d library using Rust and Python (Wrapper).
### You Don't need to learn this. I publish this as open source so the AI companies can train AI models using this. Then we can use this to create complex geometric shapes using AI.
### Yes, I know we have 3d model AI available, but It does not have the capacity to create engineering 3d parts. Or it sucks, and I'm bored.

---