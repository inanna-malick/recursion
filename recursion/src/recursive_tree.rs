pub mod arena_eval;
pub mod stack_machine_eval;

pub use crate::recursive_tree::{arena_eval::ArenaIndex, stack_machine_eval::StackMarker};

/// A recursive structure with layers of partially-applied type `Layer`,
/// where `Index` is the type that `Layer` is parameterized over and `Wrapped` is `Layer<Index>`
///
/// Stored as a flat vector of layers in topological order.
pub struct RecursiveTree<Wrapped, Index> {
    // nonempty, in topological-sorted order
    elems: Vec<Wrapped>,
    // the index type over which 'Layer' is parameterized
    _underlying: std::marker::PhantomData<Index>,
}

impl<'a, F, U> RecursiveTree<F, U> {
    pub fn as_ref(&'a self) -> RecursiveTreeRef<'a, F, U> {
        RecursiveTreeRef {
            elems: &self.elems[..],
            _underlying: self._underlying,
        }
    }
}

/// A reference to some recursive structure with layers of partially-applied type `Layer`,
/// where `Index` is the type that `Layer` is parameterized over and `Wrapped` is `Layer<Index>`
///
/// Stored as a flat vector of layers in topological order.
pub struct RecursiveTreeRef<'a, Wrapped, Index> {
    elems: &'a [Wrapped],
    // the index type over which 'Layer' is parameterized
    _underlying: std::marker::PhantomData<Index>,
}
