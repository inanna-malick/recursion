pub mod block_allocation;
pub mod dfs_stack_machine;

/// Generic struct used to represent a recursive structure of some type F<U>
pub struct RecursiveTree<F, U> {
    // nonempty, in topological-sorted order
    elems: Vec<F>,
    // the type over which 'F' is parameterized
    _underlying: std::marker::PhantomData<U>, // TODO: a better name is required than F<U>
}

impl<'a, F, U> RecursiveTree<F, U> {
    pub fn as_ref(&'a self) -> RecursiveTreeRef<'a, F, U> {
        RecursiveTreeRef {
            elems: &self.elems[..],
            _underlying: self._underlying,
        }
    }
}

/// Generic struct used to represent a refernce to a recursive structure of some type F<usize>
pub struct RecursiveTreeRef<'a, F, U> {
    // nonempty, in topological-sorted order
    elems: &'a [F],
    _underlying: std::marker::PhantomData<U>,
}
