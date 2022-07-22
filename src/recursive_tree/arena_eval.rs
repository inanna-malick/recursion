use std::collections::VecDeque;
use std::mem::MaybeUninit;

use futures::future::BoxFuture;
use futures::FutureExt;

use crate::functor::Functor;
use crate::recursive::{Collapse, Expand, ExpandAsync};
use crate::recursive_tree::{RecursiveTree, RecursiveTreeRef};

/// Used to mark structures stored in an 'RecursiveTree<Layer<ArenaIndex>, ArenaIndex>'
/// 
/// Has the same memory cost as a boxed pointer and provides the fastest
/// 'Collapse::collapse_layers' implementation
#[derive(Debug, Clone, Copy)]
pub struct ArenaIndex(usize);

impl ArenaIndex {
    fn head() -> Self {
        ArenaIndex(0)
    }
}

impl<A, U, O: Functor<ArenaIndex, Unwrapped = A, To = U>> Expand<A, O>
    for RecursiveTree<U, ArenaIndex>
{
    fn expand_layers<F: Fn(A) -> O>(a: A, generate_layer: F) -> Self {
        let mut frontier = VecDeque::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop_front() {
            let node = generate_layer(seed);

            let node = node.fmap(|aa| {
                frontier.push_back(aa);
                // idx of pointed-to element determined from frontier + elems size
                ArenaIndex(elems.len() + frontier.len())
            });

            elems.push(node);
        }

        Self {
            elems,
            _underlying: std::marker::PhantomData,
        }
    }
}

impl<A, U: Send, O: Functor<ArenaIndex, Unwrapped = A, To = U>> ExpandAsync<A, O>
    for RecursiveTree<U, ArenaIndex>
{
    fn expand_layers_async<
        'a,
        E: Send + 'a,
        F: Fn(A) -> BoxFuture<'a, Result<O, E>> + Send + Sync + 'a,
    >(
        seed: A,
        generate_layer: F,
    ) -> BoxFuture<'a, Result<Self, E>>
    where
        Self: Sized,
        U: Send,
        A: Send + 'a,
    {
        async move {
            let mut frontier = VecDeque::from([seed]);
            let mut elems = vec![];

            // unfold to build a vec of elems while preserving topo order
            while let Some(seed) = frontier.pop_front() {
                let layer = generate_layer(seed).await?;

                let layer = layer.fmap(|aa| {
                    frontier.push_back(aa);
                    // idx of pointed-to element determined from frontier + elems size
                    ArenaIndex(elems.len() + frontier.len())
                });

                elems.push(layer);
            }

            Ok(Self {
                elems,
                _underlying: std::marker::PhantomData,
            })
        }
        .boxed()
    }
}

impl<A, O, U: Functor<A, To = O, Unwrapped = ArenaIndex>> Collapse<A, O>
    for RecursiveTree<U, ArenaIndex>
{
    // TODO: 'checked' compile flag to control whether this gets a vec of maybeuninit or a vec of Option w/ unwrap
    fn collapse_layers<F: FnMut(O) -> A>(self, mut fold_layer: F) -> A {
        let mut results = std::iter::repeat_with(|| MaybeUninit::<A>::uninit())
            .take(self.elems.len())
            .collect::<Vec<_>>();

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it, also we know it's there so unsafe is fine
                let node = node.fmap(|ArenaIndex(x)| unsafe {
                    let maybe_uninit =
                        std::mem::replace(results.get_unchecked_mut(x), MaybeUninit::uninit());
                    maybe_uninit.assume_init()
                });
                fold_layer(node)
            };
            results[idx].write(alg_res);
        }

        unsafe {
            let maybe_uninit = std::mem::replace(
                results.get_unchecked_mut(ArenaIndex::head().0),
                MaybeUninit::uninit(),
            );
            maybe_uninit.assume_init()
        }
    }
}

impl<'a, A, O: 'a, U> Collapse<A, O> for RecursiveTreeRef<'a, U, ArenaIndex>
where
    &'a U: Functor<A, To = O, Unwrapped = ArenaIndex>,
{
    // TODO: 'checked' compile flag to control whether this gets a vec of maybeuninit or a vec of Option w/ unwrap
    fn collapse_layers<F: FnMut(O) -> A>(self, mut fold_layer: F) -> A {
        let mut results = std::iter::repeat_with(|| MaybeUninit::<A>::uninit())
            .take(self.elems.len())
            .collect::<Vec<_>>();

        for (idx, node) in self.elems.iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it, also we know it's there so unsafe is fine
                let node = node.fmap(|ArenaIndex(x)| unsafe {
                    let maybe_uninit =
                        std::mem::replace(results.get_unchecked_mut(x), MaybeUninit::uninit());
                    maybe_uninit.assume_init()
                });
                fold_layer(node)
            };
            results[idx].write(alg_res);
        }

        unsafe {
            let maybe_uninit = std::mem::replace(
                results.get_unchecked_mut(ArenaIndex::head().0),
                MaybeUninit::uninit(),
            );
            maybe_uninit.assume_init()
        }
    }
}
