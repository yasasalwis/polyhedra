//! SDF manipulation nodes — shape morphing and spatial patterns.
//!
//! These nodes wrap an existing SDF and modify how space is sampled before
//! evaluating it, producing rounding, hollowing, mirroring, and repetition.
//!
//! # Modules
//!
//! - [`morph`]   — `OffsetNode` (fillet / rounding), `ShellNode` (hollow),
//!                 `ElongateNode` (stretch)
//! - [`pattern`] — `MirrorNode`, `RepeatLinearNode`, `RepeatCircularNode`

pub mod morph;
pub mod pattern;

pub use morph::{ElongateNode, OffsetNode, ShellNode};
pub use pattern::{MirrorAxis, MirrorNode, RepeatCircularNode, RepeatLinearNode};
