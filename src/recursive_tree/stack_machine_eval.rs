use crate::{
    functor::Functor,
    recursive::{Collapse, Expand},
    recursive_tree::{RecursiveTree, RecursiveTreeRef},
};



/// Used to mark structures that are expanded via depth first traversal and consumed via stack machine
/// This is a zero-size marker type and has the lowest memory cost (lower than boxed pointers)
/// at the cost of a slightly slower 'Collapse::collapse_layers' fn speed
/// 
/// NOTE: adds hard requirement, functor traversal order MUST be constant and arity must not change
#[derive(Debug, Clone, Copy)]
pub struct StackMarker;

impl<A, U, O: Functor<StackMarker, Unwrapped = A, To = U>> Expand<A, O>
    for RecursiveTree<U, StackMarker>
{
    fn expand_layers<F: Fn(A) -> O>(a: A, generate_layer: F) -> Self {
        let mut frontier = Vec::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop() {
            let layer = generate_layer(seed);

            let mut topush = Vec::new();
            let layer = layer.fmap(|aa| {
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

impl<A, O, U: Functor<A, To = O, Unwrapped = StackMarker>> Collapse<A, O>
    for RecursiveTree<U, StackMarker>
{
    fn collapse_layers<F: FnMut(O) -> A>(self, mut fold_layer: F) -> A {
        let mut result_stack = Vec::new();

        for layer in self.elems.into_iter() {
            // each layer is only referenced once so just remove it, also we know it's there so unwrap is fine
            let layer = layer.fmap(|_| result_stack.pop().unwrap());

            result_stack.push(fold_layer(layer));
        }

        result_stack.pop().unwrap()
    }
}

impl<'a, A, O: 'a, U> Collapse<A, O> for RecursiveTreeRef<'a, U, StackMarker>
where
    &'a U: Functor<A, To = O, Unwrapped = StackMarker>,
{
    fn collapse_layers<F: FnMut(O) -> A>(self, mut fold_layer: F) -> A {
        let mut result_stack = Vec::with_capacity(32);

        for layer in self.elems.iter() {
            let layer = layer.fmap(|_| result_stack.pop().unwrap());

            result_stack.push(fold_layer(layer));
        }

        result_stack.pop().unwrap()
    }
}
