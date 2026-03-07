//! Unit system for polyhedra.
//!
//! All internal geometry is stored in **millimetres** (f32).
//! This module converts from any supported unit to mm.

/// Millimetres → mm (identity).
pub const MM: f32 = 1.0;
/// Centimetres → mm.
pub const CM: f32 = 10.0;
/// Metres → mm.
pub const M: f32 = 1_000.0;
/// Inches → mm (1 in = 25.4 mm).
pub const IN: f32 = 25.4;
/// Feet → mm (1 ft = 304.8 mm).
pub const FT: f32 = 304.8;

/// Convert a value expressed in `unit` to millimetres.
///
/// # Panics
/// Does not panic — unknown units are returned unchanged (treated as mm).
pub fn to_mm(value: f32, unit: &str) -> f32 {
    match unit {
        "mm"      => value * MM,
        "cm"      => value * CM,
        "m"       => value * M,
        "in"      => value * IN,
        "ft"      => value * FT,
        "degrees" => value,          // angular — caller handles
        _         => value,          // default: treat as mm
    }
}

/// Convert millimetres back to a target unit.
pub fn from_mm(value_mm: f32, unit: &str) -> f32 {
    match unit {
        "mm" => value_mm / MM,
        "cm" => value_mm / CM,
        "m"  => value_mm / M,
        "in" => value_mm / IN,
        "ft" => value_mm / FT,
        _    => value_mm,
    }
}

/// Plain-English arithmetic on top of numeric values.
///
/// Supports: `twice`, `half`, `plus <n><unit>`, `minus <n><unit>`,
/// `<n> * <expr>`. The `.polyh` parser calls these after resolving
/// variable values.
pub fn apply_arithmetic(base: f32, op: &str, operand: f32) -> f32 {
    match op {
        "twice"  => base * 2.0,
        "half"   => base / 2.0,
        "plus"   => base + operand,
        "minus"  => base - operand,
        "times"  => base * operand,
        "divide" => base / operand,
        _        => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn mm_identity() {
        assert_abs_diff_eq!(to_mm(10.0, "mm"), 10.0, epsilon = 1e-4);
    }

    #[test]
    fn cm_to_mm() {
        assert_abs_diff_eq!(to_mm(1.0, "cm"), 10.0, epsilon = 1e-4);
    }

    #[test]
    fn m_to_mm() {
        assert_abs_diff_eq!(to_mm(1.0, "m"), 1000.0, epsilon = 1e-4);
    }

    #[test]
    fn inch_to_mm() {
        assert_abs_diff_eq!(to_mm(1.0, "in"), 25.4, epsilon = 1e-4);
    }

    #[test]
    fn ft_to_mm() {
        assert_abs_diff_eq!(to_mm(1.0, "ft"), 304.8, epsilon = 1e-3);
    }

    #[test]
    fn round_trip_cm() {
        let mm = to_mm(5.0, "cm");
        assert_abs_diff_eq!(from_mm(mm, "cm"), 5.0, epsilon = 1e-4);
    }

    #[test]
    fn arithmetic_twice() {
        assert_abs_diff_eq!(apply_arithmetic(10.0, "twice", 0.0), 20.0, epsilon = 1e-4);
    }

    #[test]
    fn arithmetic_half() {
        assert_abs_diff_eq!(apply_arithmetic(10.0, "half", 0.0), 5.0, epsilon = 1e-4);
    }

    #[test]
    fn arithmetic_plus() {
        assert_abs_diff_eq!(apply_arithmetic(10.0, "plus", 5.0), 15.0, epsilon = 1e-4);
    }
}
