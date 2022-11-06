//! Recursive structure stored using a compact stack machine representation
//! Collapsed via stack machine evaluation.
//!
use crate::{
    map_layer::MapLayer,
    recursive::{Collapse, Expand},
    recursive_tree::{RecursiveTree, RecursiveTreeRef},
};

/// Used to mark structures that are expanded via depth first traversal and consumed via stack machine
/// This is a zero-size marker type and has the lowest memory cost (lower than boxed pointers)
/// at the cost of a slightly slower 'Collapse::collapse_layers' fn speed
///
/// NOTE: adds hard requirement, map_layer traversal order MUST be constant and arity must not change
#[derive(Debug, Clone, Copy)]
pub struct StackMarker;

impl<A, U, O: MapLayer<Unwrapped = A, Layer<StackMarker> = U>> Expand<A, O>
    for RecursiveTree<U, StackMarker>
{
    fn expand_layers<F: Fn(A) -> O>(a: A, generate_layer: F) -> Self {
        let mut frontier = Vec::from([a]);
        let mut elems = vec![];

        // expand to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop() {
            let layer = generate_layer(seed);

            let mut topush = Vec::new();
            let layer = layer.map_layer(|aa| {
                topush.push(aa);
                StackMarker
            });
            frontier.extend(topush.into_iter().rev());

            elems.push(layer);
        }

        elems.reverse();

        Self {
            elems,
            _underlying: std::marker::PhantomData,
        }
    }
}

impl<A, O, U: MapLayer<Layer<A> = O, Unwrapped = StackMarker>> Collapse<A, O>
    for RecursiveTree<U, StackMarker>
{
    fn collapse_layers<F: FnMut(O) -> A>(self, mut collapse_layer: F) -> A {
        let mut result_stack = Vec::new();

        for layer in self.elems.into_iter() {
            // each layer is only referenced once so just remove it, also we know it's there so unwrap is fine
            let layer = layer.map_layer(|_| result_stack.pop().unwrap());

            result_stack.push(collapse_layer(layer));
        }

        result_stack.pop().unwrap()
    }
}

impl<'a, A, O: 'a, U> Collapse<A, O> for RecursiveTreeRef<'a, U, StackMarker>
where
    &'a U: MapLayer<Layer<A> = O, Unwrapped = StackMarker>,
{
    fn collapse_layers<F: FnMut(O) -> A>(self, mut collapse_layer: F) -> A {
        let mut result_stack = Vec::with_capacity(32);

        for layer in self.elems.iter() {
            let layer = layer.map_layer(|_| result_stack.pop().unwrap());

            result_stack.push(collapse_layer(layer));
        }

        result_stack.pop().unwrap()
    }
}
