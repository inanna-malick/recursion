use std::collections::VecDeque;
use std::mem::MaybeUninit;

use futures::future::BoxFuture;
use futures::FutureExt;

/// Generic struct used to represent a recursive structure of some type F<usize>
pub struct RecursiveStruct<F> {
    // nonempty, in topological-sorted order
    elems: Vec<F>,
}

impl<'a, F> RecursiveStruct<F> {
    pub fn as_ref(&'a self) -> RecursiveStructRef<'a, F> {
        RecursiveStructRef {
            elems: &self.elems[..],
        }
    }
}

/// Generic struct used to represent a refernce to a recursive structure of some type F<usize>
pub struct RecursiveStructRef<'a, F> {
    // nonempty, in topological-sorted order
    elems: &'a [F],
}

/// Support for recursion - folding a recursive structure into a single seed
pub trait Recursive<A, O> {
    fn fold<F: FnMut(O) -> A>(self, alg: F) -> A;
}

// TODO: filtered cata that has a pre-anything fn of, like, forall x F(x) -> Fx, so it can, like, drop directories or w/e by looking at 1 layer only

// answer to visitor pattern question (how to do some actions in before, some in after branches)
// my answer: do the 'before'/'filter' type stuff in ana, as the structure is built (not a great answer)

/// Support for corecursion - unfolding a recursive structure from a seed
pub trait CoRecursive<A, O> {
    fn unfold<F: Fn(A) -> O>(a: A, coalg: F) -> Self;
}

pub trait CoRecursiveAsync<A, O> {
    fn unfold_async<'a, E: Send + 'a, F: Fn(A) -> BoxFuture<'a, Result<O, E>> + Send + Sync + 'a>(
        a: A,
        coalg: F,
    ) -> BoxFuture<'a, Result<Self, E>>
    where
        Self: Sized,
        A: Send + 'a;
}

impl<A, U, O: Functor<usize, Unwrapped = A, To = U>> CoRecursive<A, O> for RecursiveStruct<U> {
    fn unfold<F: Fn(A) -> O>(a: A, coalg: F) -> Self {
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

        Self { elems }
    }
}

// TODO: depth first traversal by replacing queue with a stack and using a hashmap instead of (more compact, but inefficient) vec append
impl<A, U: Send, O: Functor<usize, Unwrapped = A, To = U>> CoRecursiveAsync<A, O>
    for RecursiveStruct<U>
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

            Ok(Self { elems })
        }
        .boxed()
    }
}

impl<A, O, U: Functor<A, To = O, Unwrapped = usize>> Recursive<A, O> for RecursiveStruct<U> {
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

impl<'a, A, O: 'a, U> Recursive<A, O> for RecursiveStructRef<'a, U>
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

pub trait Functor<B> {
    type Unwrapped;
    type To;
    /// fmap over an owned value
    fn fmap<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To;
}
