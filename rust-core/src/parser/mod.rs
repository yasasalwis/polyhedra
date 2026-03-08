//! `.polyh` DSL parser.
//!
//! Converts raw text (or a file path) into a [`PolyhFile`] AST.
//!
//! # Entry points
//! - [`parse_str`]  — parse a `&str` directly.
//! - [`parse_file`] — read a file from disk and parse it.

pub mod ast;
pub mod eval;

use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

use crate::error::{PolyhedraError, Result};
use ast::*;

// ── Pest parser ────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
struct PolyhParser;

// ── Helpers ────────────────────────────────────────────────────────────────────

fn parse_err(msg: impl Into<String>) -> PolyhedraError {
    PolyhedraError::GeometryError {
        message: msg.into(),
    }
}

fn parse_num(pair: Pair<Rule>) -> Result<f32> {
    pair.as_str()
        .parse::<f32>()
        .map_err(|_| parse_err(format!("invalid number: '{}'", pair.as_str())))
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Parse a `.polyh` source string and return the AST.
pub fn parse_str(source: &str) -> Result<PolyhFile> {
    let mut pairs = PolyhParser::parse(Rule::file, source).map_err(|e| {
        let (line, col) = match e.line_col {
            pest::error::LineColLocation::Pos((l, c)) => (l, c),
            pest::error::LineColLocation::Span((l, c), _) => (l, c),
        };
        PolyhedraError::ParseError {
            line,
            col,
            message: e.variant.message().to_string(),
        }
    })?;

    // `pairs` is a Pairs iterator whose first element is the `file` rule pair.
    // Descend into it to reach the top_level_item children.
    let file_pair = pairs
        .next()
        .expect("pest always yields at least one pair for file");
    let mut file = PolyhFile::default();
    for pair in file_pair.into_inner() {
        match pair.as_rule() {
            Rule::top_level_item => {
                let inner = pair.into_inner().next().unwrap();
                file.items.push(build_top_level(inner)?);
            }
            Rule::EOI => {}
            _ => {}
        }
    }
    Ok(file)
}

/// Read `path` from disk and parse it.
pub fn parse_file(path: impl AsRef<std::path::Path>) -> Result<PolyhFile> {
    let path = path.as_ref();
    let source = std::fs::read_to_string(path).map_err(|e| PolyhedraError::FileNotFound {
        path: path.display().to_string() + &format!(": {e}"),
    })?;
    parse_str(&source)
}

// ── Top-level ──────────────────────────────────────────────────────────────────

fn build_top_level(pair: Pair<Rule>) -> Result<TopLevelItem> {
    match pair.as_rule() {
        Rule::use_stmt => Ok(TopLevelItem::Use(
            pair.into_inner().next().unwrap().as_str().to_string(),
        )),
        Rule::define_block => Ok(TopLevelItem::Define(build_define(pair)?)),
        Rule::assemble_block => Ok(TopLevelItem::Assemble(build_assemble(pair)?)),
        Rule::export_block => Ok(TopLevelItem::Export(build_export(pair)?)),
        r => Err(parse_err(format!("unexpected top-level rule: {r:?}"))),
    }
}

// ── define block ───────────────────────────────────────────────────────────────

fn build_define(pair: Pair<Rule>) -> Result<DefineBlock> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut items = Vec::new();
    for item in inner {
        if item.as_rule() == Rule::define_body_item {
            let child = item.into_inner().next().unwrap();
            items.push(build_define_item(child)?);
        }
    }
    Ok(DefineBlock { name, items })
}

fn build_define_item(pair: Pair<Rule>) -> Result<DefineItem> {
    match pair.as_rule() {
        Rule::units_stmt => Ok(DefineItem::Units(build_unit(pair)?)),
        Rule::primitive_block => Ok(DefineItem::Primitive(build_primitive(pair)?)),
        Rule::manipulation => Ok(DefineItem::Manip(build_manipulation(pair)?)),
        r => Err(parse_err(format!("unexpected define item: {r:?}"))),
    }
}

// ── primitive block ────────────────────────────────────────────────────────────

fn build_primitive(pair: Pair<Rule>) -> Result<PrimitiveBlock> {
    let mut inner = pair.into_inner();
    let kw = inner.next().unwrap();
    let kind = match kw.as_str() {
        "cube" => PrimKind::Cube,
        "sphere" => PrimKind::Sphere,
        "cylinder" => PrimKind::Cylinder,
        "cone" => PrimKind::Cone,
        "torus" => PrimKind::Torus,
        "pyramid" => PrimKind::Pyramid,
        "prism" => PrimKind::Prism,
        other => return Err(parse_err(format!("unknown primitive: '{other}'"))),
    };
    let mut units = None;
    let mut props = Vec::new();
    let mut moves = Vec::new();
    for item in inner {
        if item.as_rule() == Rule::prim_body_item {
            let child = item.into_inner().next().unwrap();
            match child.as_rule() {
                Rule::units_stmt => {
                    units = Some(build_unit(child)?);
                }
                Rule::prop_stmt => {
                    props.push(build_prop(child)?);
                }
                Rule::move_stmt => {
                    moves.push(build_move(child)?);
                }
                _ => {}
            }
        }
    }
    Ok(PrimitiveBlock {
        kind,
        units,
        props,
        moves,
    })
}

fn build_prop(pair: Pair<Rule>) -> Result<Prop> {
    let mut inner = pair.into_inner();
    let key = inner.next().unwrap().as_str().to_string();
    let value = parse_num(inner.next().unwrap())?;
    Ok(Prop { key, value })
}

fn build_move(pair: Pair<Rule>) -> Result<MoveStmt> {
    let mut inner = pair.into_inner();
    let axis = build_axis(inner.next().unwrap())?;
    let value = parse_num(inner.next().unwrap())?;
    Ok(MoveStmt { axis, value })
}

// ── manipulation ───────────────────────────────────────────────────────────────

fn build_manipulation(pair: Pair<Rule>) -> Result<Manipulation> {
    let child = pair.into_inner().next().unwrap();
    match child.as_rule() {
        Rule::chamfer_stmt => {
            let n = parse_num(child.into_inner().next().unwrap())?;
            Ok(Manipulation::Chamfer(n))
        }
        Rule::fillet_stmt => {
            let n = parse_num(child.into_inner().next().unwrap())?;
            Ok(Manipulation::Fillet(n))
        }
        Rule::shell_stmt => {
            let n = parse_num(child.into_inner().next().unwrap())?;
            Ok(Manipulation::Shell(n))
        }
        Rule::hole_stmt => {
            let mut inner = child.into_inner();
            let diameter = parse_num(inner.next().unwrap())?;
            let depth = inner.next().map(parse_num).transpose()?;
            Ok(Manipulation::Hole { diameter, depth })
        }
        Rule::thread_stmt => {
            let mut inner = child.into_inner();
            let diameter = parse_num(inner.next().unwrap())?;
            let pitch = inner.next().map(parse_num).transpose()?;
            Ok(Manipulation::Thread { diameter, pitch })
        }
        Rule::pattern_stmt => {
            let mut inner = child.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let count = parse_num(inner.next().unwrap())?;
            Ok(Manipulation::Pattern { name, count })
        }
        r => Err(parse_err(format!("unexpected manipulation: {r:?}"))),
    }
}

// ── assemble block ─────────────────────────────────────────────────────────────

fn build_assemble(pair: Pair<Rule>) -> Result<AssembleBlock> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut items = Vec::new();
    for item in inner {
        if item.as_rule() == Rule::assemble_body_item {
            let child = item.into_inner().next().unwrap();
            match child.as_rule() {
                Rule::units_stmt => items.push(AssembleItem::Units(build_unit(child)?)),
                Rule::assemble_op => {
                    let op_pair = child.into_inner().next().unwrap();
                    items.push(AssembleItem::Op(build_assemble_op(op_pair)?));
                }
                Rule::manipulation => {
                    items.push(AssembleItem::Manip(build_manipulation(child)?));
                }
                _ => {}
            }
        }
    }
    Ok(AssembleBlock { name, items })
}

fn build_assemble_op(pair: Pair<Rule>) -> Result<AssembleOp> {
    match pair.as_rule() {
        Rule::place_stmt => {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let at = build_position(inner.next().unwrap())?;
            Ok(AssembleOp::Place { name, at })
        }
        Rule::cut_stmt => {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let at = build_position(inner.next().unwrap())?;
            let pointing = inner.next().map(build_direction).transpose()?;
            Ok(AssembleOp::Cut { name, at, pointing })
        }
        Rule::join_stmt => {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let at = build_position(inner.next().unwrap())?;
            Ok(AssembleOp::Join { name, at })
        }
        Rule::intersect_stmt => {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let at = build_position(inner.next().unwrap())?;
            Ok(AssembleOp::Intersect { name, at })
        }
        Rule::subtract_stmt => {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let at = build_position(inner.next().unwrap())?;
            Ok(AssembleOp::Subtract { name, at })
        }
        r => Err(parse_err(format!("unexpected assemble op: {r:?}"))),
    }
}

// ── export block ───────────────────────────────────────────────────────────────

fn build_export(pair: Pair<Rule>) -> Result<ExportBlock> {
    let mut format = None;
    let mut quality = None;
    let mut file = None;
    for item in pair.into_inner() {
        if item.as_rule() == Rule::export_body_item {
            let child = item.into_inner().next().unwrap();
            match child.as_rule() {
                Rule::export_format_stmt => {
                    format = Some(child.into_inner().next().unwrap().as_str().to_lowercase());
                }
                Rule::export_quality_stmt => {
                    quality = Some(
                        match child
                            .into_inner()
                            .next()
                            .unwrap()
                            .as_str()
                            .to_lowercase()
                            .as_str()
                        {
                            "low" => Quality::Low,
                            "high" => Quality::High,
                            "ultra" => Quality::Ultra,
                            _ => Quality::Medium,
                        },
                    );
                }
                Rule::export_file_stmt => {
                    // Collect all tokens inside file_stmt and join them.
                    // Grammar: "file" ~ (string | ident ~ ("." ~ ident)*)
                    let s = child
                        .into_inner()
                        .map(|p| p.as_str().to_string())
                        .collect::<Vec<_>>()
                        .join("");
                    // Strip surrounding quotes from a string literal if present.
                    let s = s.trim_matches('"').to_string();
                    file = Some(s);
                }
                _ => {}
            }
        }
    }
    Ok(ExportBlock {
        format,
        quality,
        file,
    })
}

// ── Leaf helpers ───────────────────────────────────────────────────────────────

fn build_unit(pair: Pair<Rule>) -> Result<Unit> {
    // pair is `units_stmt`; first inner child is `unit_kw`
    let kw = pair.into_inner().next().unwrap();
    match kw.as_str() {
        "mm" => Ok(Unit::Mm),
        "cm" => Ok(Unit::Cm),
        "m" => Ok(Unit::M),
        "in" => Ok(Unit::In),
        "ft" => Ok(Unit::Ft),
        other => Err(parse_err(format!("unknown unit: '{other}'"))),
    }
}

fn build_axis(pair: Pair<Rule>) -> Result<Axis> {
    match pair.as_str() {
        "x" => Ok(Axis::X),
        "y" => Ok(Axis::Y),
        "z" => Ok(Axis::Z),
        other => Err(parse_err(format!("unknown axis: '{other}'"))),
    }
}

/// Parse a `position` rule pair.
///
/// Grammar: `position = { "origin" | ("(" ~ number ~ "," ~ number ~ "," ~ number ~ ")") }`
///
/// When matched as "origin" there are no inner pairs.
/// When matched as `(x, y, z)` there are exactly 3 `number` inner pairs.
fn build_position(pair: Pair<Rule>) -> Result<Position> {
    let mut nums = pair.into_inner();
    let first = nums.next();
    match first {
        None => Ok(Position::Origin), // "origin" — no inner pairs
        Some(x_pair) => {
            let x = parse_num(x_pair)?;
            let y = parse_num(nums.next().ok_or_else(|| parse_err("position missing y"))?)?;
            let z = parse_num(nums.next().ok_or_else(|| parse_err("position missing z"))?)?;
            Ok(Position::Coords(x, y, z))
        }
    }
}

fn build_direction(pair: Pair<Rule>) -> Result<Direction> {
    match pair.as_str() {
        "up" => Ok(Direction::Up),
        "down" => Ok(Direction::Down),
        "left" => Ok(Direction::Left),
        "right" => Ok(Direction::Right),
        "+x" => Ok(Direction::PosX),
        "-x" => Ok(Direction::NegX),
        "+y" => Ok(Direction::PosY),
        "-y" => Ok(Direction::NegY),
        "+z" => Ok(Direction::PosZ),
        "-z" => Ok(Direction::NegZ),
        other => Err(parse_err(format!("unknown direction: '{other}'"))),
    }
}
