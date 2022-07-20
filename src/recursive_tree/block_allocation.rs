use std::collections::VecDeque;
use std::mem::MaybeUninit;

use futures::future::BoxFuture;
use futures::FutureExt;

use crate::functor::Functor;
use crate::recursive::{Foldable, Generatable, GeneratableAsync};
use crate::recursive_tree::{RecursiveTree, RecursiveTreeRef};

impl<A, U, O: Functor<usize, Unwrapped = A, To = U>> Generatable<A, O> for RecursiveTree<U, usize> {
    fn generate_layer<F: Fn(A) -> O>(a: A, coalg: F) -> Self {
        let mut frontier = VecDeque::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop_front() {
            let node = coalg(seed);

            let node = node.fmap(|aa| {
                frontier.push_back(aa);
                // idx of pointed-to element determined from frontier + elems size
                elems.len() + frontier.len()
            });

            elems.push(node);
        }

        Self {
            elems,
            _underlying: std::marker::PhantomData,
        }
    }
}

impl<A, U: Send, O: Functor<usize, Unwrapped = A, To = U>> GeneratableAsync<A, O>
    for RecursiveTree<U, usize>
{
    fn unfold_async<'a, E: Send + 'a, F: Fn(A) -> BoxFuture<'a, Result<O, E>> + Send + Sync + 'a>(
        a: A,
        coalg: F,
    ) -> BoxFuture<'a, Result<Self, E>>
    where
        Self: Sized,
        U: Send,
        A: Send + 'a,
    {
        async move {
            let mut frontier = VecDeque::from([a]);
            let mut elems = vec![];

            // unfold to build a vec of elems while preserving topo order
            while let Some(seed) = frontier.pop_front() {
                let node = coalg(seed).await?;

                let node = node.fmap(|aa| {
                    frontier.push_back(aa);
                    // idx of pointed-to element determined from frontier + elems size
                    elems.len() + frontier.len()
                });

                elems.push(node);
            }

            Ok(Self {
                elems,
                _underlying: std::marker::PhantomData,
            })
        }
        .boxed()
    }
}

impl<A, O, U: Functor<A, To = O, Unwrapped = usize>> Foldable<A, O> for RecursiveTree<U, usize> {
    // TODO: 'checked' compile flag to control whether this gets a vec of maybeuninit or a vec of Option w/ unwrap
    fn fold<F: FnMut(O) -> A>(self, mut alg: F) -> A {
        let mut results = std::iter::repeat_with(|| MaybeUninit::<A>::uninit())
            .take(self.elems.len())
            .collect::<Vec<_>>();

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it, also we know it's there so unsafe is fine
                let node = node.fmap(|x| unsafe {
                    let maybe_uninit =
                        std::mem::replace(results.get_unchecked_mut(x), MaybeUninit::uninit());
                    maybe_uninit.assume_init()
                });
                alg(node)
            };
            results[idx].write(alg_res);
        }

        unsafe {
            let maybe_uninit =
                std::mem::replace(results.get_unchecked_mut(0), MaybeUninit::uninit());
            maybe_uninit.assume_init()
        }
    }
}

impl<'a, A, O: 'a, U> Foldable<A, O> for RecursiveTreeRef<'a, U, usize>
where
    &'a U: Functor<A, To = O, Unwrapped = usize>,
{
    // TODO: 'checked' compile flag to control whether this gets a vec of maybeuninit or a vec of Option w/ unwrap
    fn fold<F: FnMut(O) -> A>(self, mut alg: F) -> A {
        let mut results = std::iter::repeat_with(|| MaybeUninit::<A>::uninit())
            .take(self.elems.len())
            .collect::<Vec<_>>();

        for (idx, node) in self.elems.iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it, also we know it's there so unsafe is fine
                let node = node.fmap(|x| unsafe {
                    let maybe_uninit =
                        std::mem::replace(results.get_unchecked_mut(x), MaybeUninit::uninit());
                    maybe_uninit.assume_init()
                });
                alg(node)
            };
            results[idx].write(alg_res);
        }

        unsafe {
            let maybe_uninit =
                std::mem::replace(results.get_unchecked_mut(0), MaybeUninit::uninit());
            maybe_uninit.assume_init()
        }
    }
}
