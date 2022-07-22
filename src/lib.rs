pub mod functor;
pub mod recursive;
pub mod recursive_tree;
pub mod stack_machine_lazy;
// using cfg flag to make expr examples available in a benchmark context
#[cfg(any(test, feature = "expr_example"))]
pub mod examples;

pub use crate::recursive::{Collapse, Expand, ExpandAsync};