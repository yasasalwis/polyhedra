//! Shape type enum — mirrors the integer constants in `polyhedra/constants.py`.
//!
//! These IDs travel across the Python↔Rust boundary: Python passes an integer
//! and Rust converts it to a `ShapeType` variant via `TryFrom<u32>`.

use crate::error::PolyhedraError;

/// Integer IDs for every primitive shape.
///
/// The discriminant values match the Python constants (`ph.CUBE`, `ph.CYLINDER`,
/// etc.) defined in `polyhedra/constants.py`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ShapeType {
    Cube = 0,
    Cylinder = 1,
    Sphere = 2,
    Cone = 3,
    Torus = 4,
    Pyramid = 5,
    Prism = 6,
}

impl TryFrom<u32> for ShapeType {
    type Error = PolyhedraError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ShapeType::Cube),
            1 => Ok(ShapeType::Cylinder),
            2 => Ok(ShapeType::Sphere),
            3 => Ok(ShapeType::Cone),
            4 => Ok(ShapeType::Torus),
            5 => Ok(ShapeType::Pyramid),
            6 => Ok(ShapeType::Prism),
            other => Err(PolyhedraError::GeometryError {
                message: format!("unknown shape type id: {other}"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_all_variants() {
        let variants = [
            (0, ShapeType::Cube),
            (1, ShapeType::Cylinder),
            (2, ShapeType::Sphere),
            (3, ShapeType::Cone),
            (4, ShapeType::Torus),
            (5, ShapeType::Pyramid),
            (6, ShapeType::Prism),
        ];
        for (id, expected) in variants {
            let got = ShapeType::try_from(id).expect("known id should convert");
            assert_eq!(got, expected);
        }
    }

    #[test]
    fn unknown_id_returns_error() {
        assert!(ShapeType::try_from(99).is_err());
    }
}
