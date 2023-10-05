mod frame;
mod recursive;

#[cfg(feature = "experimental")]
pub mod experimental;

pub use frame::{MappableFrame, PartiallyApplied, expand_and_collapse};
pub use recursive::Collapsible;
pub use recursive::Expandable;
