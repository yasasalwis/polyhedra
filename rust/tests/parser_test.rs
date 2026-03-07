//! Integration tests for the `.polyh` DSL parser.
//!
//! Tests cover all grammar constructs without requiring file I/O — each test
//! feeds a raw `&str` to `parse_str` and inspects the resulting AST.

use _core::parser::parse_str;
use _core::parser::ast::*;

// ── Trivial ────────────────────────────────────────────────────────────────────

#[test]
fn empty_file_is_ok() {
    let f = parse_str("").unwrap();
    assert!(f.items.is_empty());
}

#[test]
fn comments_only_is_ok() {
    let src = "# this is a comment\n# another one\n";
    let f = parse_str(src).unwrap();
    assert!(f.items.is_empty());
}

// ── use statement ──────────────────────────────────────────────────────────────

#[test]
fn use_stmt_parsed() {
    let f = parse_str("use hello").unwrap();
    assert_eq!(f.items, vec![TopLevelItem::Use("hello".into())]);
}

#[test]
fn multiple_use_stmts() {
    let f = parse_str("use a\nuse b\nuse c").unwrap();
    assert_eq!(f.items.len(), 3);
}

// ── define block — primitives ──────────────────────────────────────────────────

#[test]
fn define_cube_basic() {
    let src = r#"
define box
  cube
    width 10
    depth 5
    height 3
  end
end
"#;
    let f = parse_str(src).unwrap();
    let def = f.defines().next().expect("no define block");
    assert_eq!(def.name, "box");
    assert_eq!(def.items.len(), 1);
    if let DefineItem::Primitive(p) = &def.items[0] {
        assert_eq!(p.kind, PrimKind::Cube);
        assert_eq!(p.props.len(), 3);
        assert_eq!(p.props[0], Prop { key: "width".into(),  value: 10.0 });
        assert_eq!(p.props[1], Prop { key: "depth".into(),  value: 5.0 });
        assert_eq!(p.props[2], Prop { key: "height".into(), value: 3.0 });
    } else {
        panic!("expected Primitive item");
    }
}

#[test]
fn define_sphere_with_units() {
    let src = r#"
define ball
  sphere
    units mm
    radius 7.5
  end
end
"#;
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    if let DefineItem::Primitive(p) = &def.items[0] {
        assert_eq!(p.kind,  PrimKind::Sphere);
        assert_eq!(p.units, Some(Unit::Mm));
        assert_eq!(p.props[0].value, 7.5);
    } else {
        panic!("expected Primitive");
    }
}

#[test]
fn define_cylinder() {
    let src = "define cyl\n  cylinder\n    radius 3\n    height 20\n  end\nend";
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    if let DefineItem::Primitive(p) = &def.items[0] {
        assert_eq!(p.kind, PrimKind::Cylinder);
        assert_eq!(p.props.len(), 2);
    } else { panic!() }
}

#[test]
fn define_cone() {
    let src = "define c\n  cone\n    radius 4\n    height 10\n  end\nend";
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    if let DefineItem::Primitive(p) = &def.items[0] {
        assert_eq!(p.kind, PrimKind::Cone);
    } else { panic!() }
}

#[test]
fn define_torus() {
    let src = "define t\n  torus\n    major 10\n    minor 2\n  end\nend";
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    if let DefineItem::Primitive(p) = &def.items[0] {
        assert_eq!(p.kind, PrimKind::Torus);
        assert_eq!(p.props[0].key, "major");
    } else { panic!() }
}

#[test]
fn define_pyramid() {
    let src = "define py\n  pyramid\n    base 6\n    height 12\n  end\nend";
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    if let DefineItem::Primitive(p) = &def.items[0] {
        assert_eq!(p.kind, PrimKind::Pyramid);
    } else { panic!() }
}

#[test]
fn define_prism() {
    let src = "define pr\n  prism\n    sides 6\n    radius 5\n    height 8\n  end\nend";
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    if let DefineItem::Primitive(p) = &def.items[0] {
        assert_eq!(p.kind, PrimKind::Prism);
        assert_eq!(p.props[0], Prop { key: "sides".into(), value: 6.0 });
    } else { panic!() }
}

// ── move statements ────────────────────────────────────────────────────────────

#[test]
fn primitive_with_move() {
    let src = r#"
define part
  cube
    width 10
    move x 5
    move z -3
  end
end
"#;
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    if let DefineItem::Primitive(p) = &def.items[0] {
        assert_eq!(p.moves.len(), 2);
        assert_eq!(p.moves[0], MoveStmt { axis: Axis::X, value: 5.0  });
        assert_eq!(p.moves[1], MoveStmt { axis: Axis::Z, value: -3.0 });
    } else { panic!() }
}

// ── Manipulations ──────────────────────────────────────────────────────────────

#[test]
fn chamfer_manipulation() {
    let src = "define x\n  cube\n    width 10\n  end\n  chamfer 1.5\nend";
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    assert_eq!(def.items.len(), 2);
    assert_eq!(def.items[1], DefineItem::Manip(Manipulation::Chamfer(1.5)));
}

#[test]
fn fillet_manipulation() {
    let src = "define x\n  sphere\n    radius 5\n  end\n  fillet 0.5\nend";
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    assert_eq!(def.items[1], DefineItem::Manip(Manipulation::Fillet(0.5)));
}

#[test]
fn shell_manipulation() {
    let src = "define box_shell\n  cube\n    width 20\n  end\n  shell 2\nend";
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    assert_eq!(def.items[1], DefineItem::Manip(Manipulation::Shell(2.0)));
}

#[test]
fn hole_manipulation_with_depth() {
    let src = "define x\n  cube\n    width 10\n  end\n  hole 3.2 15\nend";
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    assert_eq!(
        def.items[1],
        DefineItem::Manip(Manipulation::Hole { diameter: 3.2, depth: Some(15.0) })
    );
}

#[test]
fn hole_manipulation_without_depth() {
    let src = "define x\n  cube\n    width 10\n  end\n  hole 3.2\nend";
    let f = parse_str(src).unwrap();
    let def = f.defines().next().unwrap();
    assert_eq!(
        def.items[1],
        DefineItem::Manip(Manipulation::Hole { diameter: 3.2, depth: None })
    );
}

// ── assemble block ─────────────────────────────────────────────────────────────

#[test]
fn assemble_place_at_origin() {
    let src = r#"
assemble widget
  place base at origin
end
"#;
    let f = parse_str(src).unwrap();
    let asm = f.assemblies().next().unwrap();
    assert_eq!(asm.name, "widget");
    assert_eq!(asm.items.len(), 1);
    if let AssembleItem::Op(AssembleOp::Place { name, at }) = &asm.items[0] {
        assert_eq!(name, "base");
        assert_eq!(*at, Position::Origin);
    } else {
        panic!("expected Place op");
    }
}

#[test]
fn assemble_place_at_coords() {
    let src = "assemble a\n  place part at (1, 2, 3)\nend";
    let f = parse_str(src).unwrap();
    let asm = f.assemblies().next().unwrap();
    if let AssembleItem::Op(AssembleOp::Place { at, .. }) = &asm.items[0] {
        assert_eq!(*at, Position::Coords(1.0, 2.0, 3.0));
    } else { panic!() }
}

#[test]
fn assemble_cut_with_pointing() {
    let src = r#"
assemble drilled
  place block at origin
  cut hole at (0, 0, 0) pointing up
end
"#;
    let f = parse_str(src).unwrap();
    let asm = f.assemblies().next().unwrap();
    if let AssembleItem::Op(AssembleOp::Cut { name, at, pointing }) = &asm.items[1] {
        assert_eq!(name, "hole");
        assert_eq!(*at, Position::Coords(0.0, 0.0, 0.0));
        assert_eq!(*pointing, Some(Direction::Up));
    } else {
        panic!("expected Cut op");
    }
}

#[test]
fn assemble_cut_without_pointing() {
    let src = "assemble a\n  place b at origin\n  cut c at origin\nend";
    let f = parse_str(src).unwrap();
    let asm = f.assemblies().next().unwrap();
    if let AssembleItem::Op(AssembleOp::Cut { pointing, .. }) = &asm.items[1] {
        assert_eq!(*pointing, None);
    } else { panic!() }
}

#[test]
fn assemble_join() {
    let src = "assemble a\n  place b at origin\n  join c at (5, 0, 0)\nend";
    let f = parse_str(src).unwrap();
    let asm = f.assemblies().next().unwrap();
    assert!(matches!(asm.items[1], AssembleItem::Op(AssembleOp::Join { .. })));
}

// ── export block ───────────────────────────────────────────────────────────────

#[test]
fn export_block_all_fields() {
    let src = r#"
export
  format stl
  quality high
  file out.stl
end
"#;
    let f = parse_str(src).unwrap();
    let exp = f.export().expect("no export block");
    assert_eq!(exp.format.as_deref(),  Some("stl"));
    assert_eq!(exp.quality,            Some(Quality::High));
    assert!(exp.file.as_deref().unwrap().contains("out"));
}

#[test]
fn export_block_format_only() {
    let src = "export\n  format obj\nend";
    let f = parse_str(src).unwrap();
    let exp = f.export().unwrap();
    assert_eq!(exp.format.as_deref(), Some("obj"));
    assert_eq!(exp.quality, None);
    assert_eq!(exp.file,    None);
}

// ── Negative cases ─────────────────────────────────────────────────────────────

#[test]
fn unknown_top_level_is_err() {
    let result = parse_str("foobar 42");
    assert!(result.is_err(), "unknown keyword should fail parsing");
}

#[test]
fn unterminated_define_is_err() {
    let result = parse_str("define foo\n  cube\n    width 10\n  end\n");
    // Missing final `end` for the define block
    assert!(result.is_err(), "unterminated define should fail");
}

// ── Combined file ──────────────────────────────────────────────────────────────

#[test]
fn combined_define_and_assemble() {
    let src = r#"
define bracket
  units mm
  cube
    width 60
    depth 40
    height 5
  end
  chamfer 1.5
end

assemble bracket_asm
  place bracket at origin
end

export
  format stl
  quality high
  file bracket.stl
end
"#;
    let f = parse_str(src).unwrap();
    assert_eq!(f.items.len(), 3);
    assert!(matches!(f.items[0], TopLevelItem::Define(_)));
    assert!(matches!(f.items[1], TopLevelItem::Assemble(_)));
    assert!(matches!(f.items[2], TopLevelItem::Export(_)));

    // units propagate to top-level define
    let def = f.defines().next().unwrap();
    assert_eq!(def.items[0], DefineItem::Units(Unit::Mm));
    // chamfer is last item
    assert_eq!(def.items[2], DefineItem::Manip(Manipulation::Chamfer(1.5)));
}

#[test]
fn negative_number_in_position() {
    let src = "assemble a\n  place b at (-25, -15, 0)\nend";
    let f = parse_str(src).unwrap();
    let asm = f.assemblies().next().unwrap();
    if let AssembleItem::Op(AssembleOp::Place { at, .. }) = &asm.items[0] {
        assert_eq!(*at, Position::Coords(-25.0, -15.0, 0.0));
    } else { panic!() }
}

#[test]
fn unit_scale_factors() {
    assert_eq!(Unit::Mm.to_mm(),  1.0);
    assert_eq!(Unit::Cm.to_mm(), 10.0);
    assert!((Unit::In.to_mm() - 25.4).abs() < 1e-6);
}
