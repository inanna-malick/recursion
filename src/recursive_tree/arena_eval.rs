//! Recursive structure that uses an arena to quickly collapse recursive structures.

use std::collections::VecDeque;
use std::mem::MaybeUninit;

use futures::future::BoxFuture;
use futures::FutureExt;

use crate::map_layer::MapLayer;
use crate::recursive::{
    Collapse, CollapseWithSubStructure, Expand, ExpandAsync,
};
use crate::recursive_tree::{RecursiveTree, RecursiveTreeRef};

/// Used to mark structures stored in an 'RecursiveTree<Layer<ArenaIndex>, ArenaIndex>'
///
/// Has the same memory cost as a boxed pointer and provides the fastest
/// 'Collapse::collapse_layers' implementation
#[derive(Debug, Clone, Copy)]
pub struct ArenaIndex(usize);

// TODO: can I implement the opposite? append single node to recursive struct?
impl ArenaIndex {
    fn head() -> Self {
        ArenaIndex(0)
    }
}

#[derive(Debug)]
pub struct RecursiveTreeRefWithOffset<'a, Wrapped> {
    recursive_tree: RecursiveTreeRef<'a, Wrapped, ArenaIndex>,
    offset: usize, // arena index offset
}

pub trait Head<'a, Unwrapped> {
    fn head(&'a self) -> Unwrapped;
}

impl<'a, Wrapped, Unwrapped> Head<'a, Unwrapped> for RecursiveTree<Wrapped, ArenaIndex>
where
    &'a Wrapped: MapLayer<RecursiveTreeRefWithOffset<'a, Wrapped>, Unwrapped = ArenaIndex, To = Unwrapped>
        + 'a,
    Unwrapped: 'a,
{
    // self -> Layer<RecursiveTreeRef>
    fn head(&'a self) -> Unwrapped {
        let head = &self.elems[0]; // invariant: always present
        head.map_layer(|idx| RecursiveTreeRefWithOffset {
            recursive_tree: RecursiveTreeRef {
                elems: &self.elems[idx.0..],
                _underlying: std::marker::PhantomData,
            },
            offset: idx.0,
        })
    }
}



impl<'a, Wrapped, Unwrapped> Head<'a, Unwrapped> for RecursiveTreeRefWithOffset<'a, Wrapped>
where
    &'a Wrapped: MapLayer<RecursiveTreeRefWithOffset<'a, Wrapped>, Unwrapped = ArenaIndex, To = Unwrapped>
        + 'a,
    Unwrapped: 'a,
{
    // self -> Layer<RecursiveTreeRef>
    fn head(&'a self) -> Unwrapped {
        let head = &self.recursive_tree.elems[0]; // invariant: always present
        head.map_layer(|idx| RecursiveTreeRefWithOffset {
            recursive_tree: RecursiveTreeRef {
                elems: &self.recursive_tree.elems[(idx.0 - self.offset)..],
                _underlying: std::marker::PhantomData,
            },
            offset: idx.0 + self.offset,
        })
    }
}

impl<A, Underlying, Wrapped> Expand<A, Wrapped> for RecursiveTree<Underlying, ArenaIndex>
where
    Wrapped: MapLayer<ArenaIndex, Unwrapped = A, To = Underlying>,
{
    fn expand_layers<F: Fn(A) -> Wrapped>(a: A, expand_layer: F) -> Self {
        let mut frontier = VecDeque::from([a]);
        let mut elems = vec![];

        // expand to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop_front() {
            let layer = expand_layer(seed);

            let layer = layer.map_layer(|aa| {
                frontier.push_back(aa);
                // idx of pointed-to element determined from frontier + elems size
                ArenaIndex(elems.len() + frontier.len())
            });

            elems.push(layer);
        }

        Self {
            elems,
            _underlying: std::marker::PhantomData,
        }
    }
}

impl<A, U: Send, O: MapLayer<ArenaIndex, Unwrapped = A, To = U>> ExpandAsync<A, O>
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

            // expand to build a vec of elems while preserving topo order
            while let Some(seed) = frontier.pop_front() {
                let layer = generate_layer(seed).await?;

                let layer = layer.map_layer(|aa| {
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

impl<A, Wrapped, Underlying> Collapse<A, Wrapped> for RecursiveTree<Underlying, ArenaIndex>
where
    Underlying: MapLayer<A, To = Wrapped, Unwrapped = ArenaIndex>,
{
    // TODO: 'checked' compile flag to control whether this gets a vec of maybeuninit or a vec of Option w/ unwrap
    fn collapse_layers<F: FnMut(Wrapped) -> A>(self, mut collapse_layer: F) -> A {
        let mut results = std::iter::repeat_with(|| MaybeUninit::<A>::uninit())
            .take(self.elems.len())
            .collect::<Vec<_>>();

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it, also we know it's there so unsafe is fine
                let node = node.map_layer(|ArenaIndex(x)| unsafe {
                    let maybe_uninit =
                        std::mem::replace(results.get_unchecked_mut(x), MaybeUninit::uninit());
                    maybe_uninit.assume_init()
                });
                collapse_layer(node)
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

// recurse with context-labeled subtree, lol what lmao how does this compile
impl<'a, A, Wrapped, Underlying> CollapseWithSubStructure<'a, A, Wrapped>
    for RecursiveTreeRef<'a, Underlying, ArenaIndex>
where
    &'a Underlying: MapLayer<
        (
            A,
            RecursiveTreeRefWithOffset<'a, Underlying>,
        ),
        To = Wrapped,
        Unwrapped = ArenaIndex,
    >,
    A: 'a,
    Wrapped: 'a, // Layer<(&A, RecursiveTreeRefWithOffset)> -> A
    Underlying: 'a,
{

    fn paramorphism<F: FnMut(Wrapped) -> A>(&self, mut collapse_layer: F) -> A {
        let mut results: Vec<Option<A>> = std::iter::repeat_with(|| None)
            .take(self.elems.len())
            .collect::<Vec<_>>();

        for (idx, node) in self.elems.iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it, also we know it's there so unsafe is fine
                let node = node.map_layer(|ArenaIndex(x)| {
                    // TODO: get ref instead of remove

                    let substructure = RecursiveTreeRefWithOffset {
                        recursive_tree: RecursiveTreeRef {
                            elems: &self.elems[x..],
                            _underlying: std::marker::PhantomData,
                        },
                        offset: x,
                    };

                    // results only used once, will need some fancy bullshit for histomorphism tho
                    (results[x].take().unwrap(), substructure)
                });
                collapse_layer(node)
            };
            results[idx] = Some(alg_res);
        }

        // doesn't preserve ordering, but at this point we're done and
        // don't care
        let mut maybe = results.swap_remove(ArenaIndex::head().0);
        maybe.take().unwrap()
    }
}

impl<'a, A, O: 'a, U> Collapse<A, O> for RecursiveTreeRef<'a, U, ArenaIndex>
where
    &'a U: MapLayer<A, To = O, Unwrapped = ArenaIndex>,
{
    fn collapse_layers<F: FnMut(O) -> A>(self, collapse_layer: F) -> A {
        RecursiveTreeRefWithOffset {
            recursive_tree: self,
            offset: 0,
        }
        .collapse_layers(collapse_layer)
    }
}

impl<'a, A, O: 'a, U> Collapse<A, O> for RecursiveTreeRefWithOffset<'a, U>
where
    &'a U: MapLayer<A, To = O, Unwrapped = ArenaIndex>,
{
    // TODO: 'checked' compile flag to control whether this gets a vec of maybeuninit or a vec of Option w/ unwrap
    fn collapse_layers<F: FnMut(O) -> A>(self, mut collapse_layer: F) -> A {
        let mut results = std::iter::repeat_with(|| MaybeUninit::<A>::uninit())
            .take(self.recursive_tree.elems.len())
            .collect::<Vec<_>>();

        for (idx, node) in self.recursive_tree.elems.iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it, also we know it's there so unsafe is fine
                let node = node.map_layer(|ArenaIndex(x)| unsafe {
                    let maybe_uninit = std::mem::replace(
                        results.get_unchecked_mut(x - self.offset),
                        MaybeUninit::uninit(),
                    );
                    maybe_uninit.assume_init()
                });
                collapse_layer(node)
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
