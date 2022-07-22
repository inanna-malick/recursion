pub mod block_allocation;
pub mod dfs_stack_machine;

/// A reference to some recursive structure with layers of partially-applied type `Layer`, 
/// where `Index` is the type that `Layer` is parameterized over and `Wrapped` is `Layer<Index>`
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
pub struct RecursiveTreeRef<'a, Wrapped, Index> {
    // the index type over which 'Layer' is parameterized
    elems: &'a [Wrapped],
    // the type over which 'F' is parameterized
    _underlying: std::marker::PhantomData<Index>,
}
