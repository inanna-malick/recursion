use std::collections::HashMap;
use std::collections::VecDeque;

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

// TODO: make O an associated type
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
    fn fold<F: FnMut(O) -> A>(self, mut alg: F) -> A {
        let mut results: HashMap<usize, A> = HashMap::with_capacity(self.elems.len());

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it
                let node = node.fmap(|x| results.remove(&x).expect("node not in result map"));
                alg(node)
            };
            results.insert(idx, alg_res);
        }

        results.remove(&0).unwrap()
    }
}

// TODO: consider using slab instead of vec for underlying RecursiveStruct

// TODO: use noop hasher impl for usize - much much faster, all usizes are unique

// IDEA - take a mutable ref - &mut self, for fold and unfold - could then use vec drain (?) - so then struct is holding ARENA instead of just the elem- 'recursion scheme evaluator type' - could own and reuse hashmap
// IDEA (cont) - if I'm repeatedly evaluating a cata I could reuse an arena? would have to grow it for each eval - can drop arena each eval and reuse allocation, can amortize to LESS THAN ONE EVAL per fold
// would use same alloc for fold/unfolds - evaluator struct tied to single <A, O>

// can use slab to impl fused fold/unfold mb - also read impl? it's interesting

// TODO - compile pass over F<slabref> to preserve recursive links

impl<'a, A, O: 'a, U> Recursive<A, O> for RecursiveStructRef<'a, U>
where
    &'a U: Functor<A, To = O, Unwrapped = usize>,
{
    fn fold<F: FnMut(O) -> A>(self, mut alg: F) -> A {
        // TODO: Vec of options - avoid pointer chasing, mb more cache friendly - no linear probing on linked list
        // TODO: implement benchmarks before implementing this? advice: better to allocate once (Vec) instead - size doesn't matter much - would only hit malloc once, would avoid having to copy each element because guaranteed no vec grow ops at ALL
        let mut results: HashMap<usize, A> = HashMap::with_capacity(self.elems.len());

        for (idx, node) in self.elems.iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it
                let node = node.fmap(|x| results.remove(&x).expect("node not in result map"));
                alg(node)
            };
            results.insert(idx, alg_res);
        }

        results.remove(&0).unwrap()
    }
}

pub trait Functor<B> {
    type Unwrapped;
    type To;
    /// fmap over an owned value
    fn fmap<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To;
}
