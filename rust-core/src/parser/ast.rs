//! Abstract Syntax Tree types for the `.polyh` DSL.
//!
//! The AST is a faithful structural representation of the grammar — it does
//! not contain any resolved geometry.  The `eval` module converts AST nodes
//! into SDF trees.

// ── Atoms ──────────────────────────────────────────────────────────────────────

/// Axis enum for move statements and revolve operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// Cardinal/direction for `pointing` clauses in cut operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

/// Unit of measurement declared by a `units` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Mm,
    Cm,
    M,
    In,
    Ft,
}

impl Unit {
    /// Scale factor relative to millimetres.
    pub fn to_mm(self) -> f32 {
        match self {
            Unit::Mm => 1.0,
            Unit::Cm => 10.0,
            Unit::M => 1_000.0,
            Unit::In => 25.4,
            Unit::Ft => 304.8,
        }
    }
}

/// 3-D position from `origin` or `(x, y, z)`.
#[derive(Debug, Clone, PartialEq)]
pub enum Position {
    Origin,
    Coords(f32, f32, f32),
}

// ── Primitive properties ───────────────────────────────────────────────────────

/// A single `key value` property line inside a primitive block.
#[derive(Debug, Clone, PartialEq)]
pub struct Prop {
    pub key: String,
    pub value: f32,
}

/// A `move axis value` statement inside a primitive block.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveStmt {
    pub axis: Axis,
    pub value: f32,
}

/// One of the named primitive keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimKind {
    Cube,
    Sphere,
    Cylinder,
    Cone,
    Torus,
    Pyramid,
    Prism,
}

/// A complete primitive block: `cube ... end`.
#[derive(Debug, Clone, PartialEq)]
pub struct PrimitiveBlock {
    pub kind: PrimKind,
    pub units: Option<Unit>,
    pub props: Vec<Prop>,
    pub moves: Vec<MoveStmt>,
}

// ── Manipulations ──────────────────────────────────────────────────────────────

/// All manipulation operations that can appear at define or assemble scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Manipulation {
    Chamfer(f32),
    Fillet(f32),
    /// Shell with a wall thickness.
    Shell(f32),
    /// Hole with diameter and optional depth.
    Hole {
        diameter: f32,
        depth: Option<f32>,
    },
    /// Thread with diameter and optional pitch.
    Thread {
        diameter: f32,
        pitch: Option<f32>,
    },
    /// Linear or circular pattern.
    Pattern {
        name: String,
        count: f32,
    },
}

// ── Define block ───────────────────────────────────────────────────────────────

/// Items that can appear in a `define` block.
#[derive(Debug, Clone, PartialEq)]
pub enum DefineItem {
    Units(Unit),
    Primitive(PrimitiveBlock),
    Manip(Manipulation),
}

/// A complete `define <name> ... end` block.
#[derive(Debug, Clone, PartialEq)]
pub struct DefineBlock {
    pub name: String,
    pub items: Vec<DefineItem>,
}

// ── Assembly operations ────────────────────────────────────────────────────────

/// A single operation inside an `assemble` block.
#[derive(Debug, Clone, PartialEq)]
pub enum AssembleOp {
    Place {
        name: String,
        at: Position,
    },
    Cut {
        name: String,
        at: Position,
        pointing: Option<Direction>,
    },
    Join {
        name: String,
        at: Position,
    },
    Intersect {
        name: String,
        at: Position,
    },
    Subtract {
        name: String,
        at: Position,
    },
}

/// Items that can appear in an `assemble` block.
#[derive(Debug, Clone, PartialEq)]
pub enum AssembleItem {
    Units(Unit),
    Op(AssembleOp),
    Manip(Manipulation),
}

/// A complete `assemble <name> ... end` block.
#[derive(Debug, Clone, PartialEq)]
pub struct AssembleBlock {
    pub name: String,
    pub items: Vec<AssembleItem>,
}

// ── Export block ───────────────────────────────────────────────────────────────

/// Export format string (lowercased).
pub type ExportFmtStr = String;

/// Quality preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Low,
    Medium,
    High,
    Ultra,
}

/// A complete `export ... end` block.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportBlock {
    pub format: Option<ExportFmtStr>,
    pub quality: Option<Quality>,
    pub file: Option<String>,
}

// ── Top-level file ─────────────────────────────────────────────────────────────

/// A single item at the top level of a `.polyh` file.
#[derive(Debug, Clone, PartialEq)]
pub enum TopLevelItem {
    Use(String),
    Define(DefineBlock),
    Assemble(AssembleBlock),
    Export(ExportBlock),
}

/// The complete parsed representation of a `.polyh` file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PolyhFile {
    pub items: Vec<TopLevelItem>,
}

impl PolyhFile {
    /// Iterate over all `define` blocks.
    pub fn defines(&self) -> impl Iterator<Item = &DefineBlock> {
        self.items.iter().filter_map(|i| {
            if let TopLevelItem::Define(d) = i {
                Some(d)
            } else {
                None
            }
        })
    }

    /// Iterate over all `assemble` blocks.
    pub fn assemblies(&self) -> impl Iterator<Item = &AssembleBlock> {
        self.items.iter().filter_map(|i| {
            if let TopLevelItem::Assemble(a) = i {
                Some(a)
            } else {
                None
            }
        })
    }

    /// Return the (first) export block if present.
    pub fn export(&self) -> Option<&ExportBlock> {
        self.items.iter().find_map(|i| {
            if let TopLevelItem::Export(e) = i {
                Some(e)
            } else {
                None
            }
        })
    }
}
