//! Stack safe and performant recursion in Rust.
//!
//! Generic utilities for expanding and collapsing user-defined recursive structures
//! of any type. Define recursive algorithms by writing functions that expand or
//! collapse a single layer of your structure.

pub mod map_layer;
pub mod recursive;
pub mod recursive_tree;
pub mod stack_machine_lazy;
// using cfg flag to make expr examples available in a benchmark context
#[cfg(any(test, feature = "expr_example"))]
pub mod examples;

pub use crate::recursive::{Collapse, Expand, ExpandAsync};
