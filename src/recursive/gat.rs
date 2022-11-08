use std::marker::PhantomData;

use super::Collapse;
use crate::map_layer::MapLayer;

pub trait FunctorToken {
    type Layer<X>;

    fn map_associated_layer<F, A, B>(input: Self::Layer<A>, f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B;
}

struct MapLayerFromFunctorToken<Layer, Unwrapped, FunctorToken>(
    Layer,
    PhantomData<Unwrapped>,
    PhantomData<FunctorToken>,
);

impl<Functor: FunctorToken, A, B> MapLayer<B>
    for MapLayerFromFunctorToken<Functor::Layer<A>, A, Functor>
{
    type Unwrapped = A;

    type To = Functor::Layer<B>;

    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
        Functor::map_associated_layer(self.0, f)
    }
}

impl<L, U, F> MapLayerFromFunctorToken<L, U, F> {
    pub fn new(x: L) -> Self {
        MapLayerFromFunctorToken(x, PhantomData, PhantomData)
    }
}

// TODO: should probably move this elsewhere mb
pub trait Recursive: FunctorToken
where
    Self: Sized,
{
    fn into_layer(self) -> Self::Layer<Self>;
}

pub trait RecursiveExt: Recursive {
    fn fold_recursive<Out, F: FnMut(Self::Layer<Out>) -> Out>(self, collapse_layer: F) -> Out;

    fn expand_and_collapse<Seed, Out>(
        seed: Seed,
        expand_layer: impl FnMut(Seed) -> Self::Layer<Seed>,
        collapse_layer: impl FnMut(Self::Layer<Out>) -> Out,
    ) -> Out;
}

impl<X> RecursiveExt for X
where
    X: Recursive,
{
    fn fold_recursive<Out, F: FnMut(Self::Layer<Out>) -> Out>(self, collapse_layer: F) -> Out {
        Self::expand_and_collapse(self, Self::into_layer, collapse_layer)
    }

    fn expand_and_collapse<Seed, Out>(
        seed: Seed,
        mut expand_layer: impl FnMut(Seed) -> Self::Layer<Seed>,
        mut collapse_layer: impl FnMut(Self::Layer<Out>) -> Out,
    ) -> Out {
        enum State<Seed, CollapsableInternal> {
            Expand(Seed),
            Collapse(CollapsableInternal),
        }

        let mut vals: Vec<Out> = vec![];
        let mut stack = vec![State::Expand(seed)];

        while let Some(item) = stack.pop() {
            match item {
                State::Expand(seed) => {
                    let node = expand_layer(seed);
                    let mut seeds = Vec::new();
                    let node = Self::map_associated_layer(node, |seed| seeds.push(seed));

                    stack.push(State::Collapse(node));
                    stack.extend(seeds.into_iter().map(State::Expand));
                }
                State::Collapse(node) => {
                    let node = Self::map_associated_layer(node, |_: ()| vals.pop().unwrap());
                    vals.push(collapse_layer(node))
                }
            };
        }
        vals.pop().unwrap()
    }
}

struct CollapseViaRecursive<X>(X);

impl<Out, R: RecursiveExt> Collapse<Out, R::Layer<Out>> for CollapseViaRecursive<R> {
    fn collapse_layers<F: FnMut(R::Layer<Out>) -> Out>(self, collapse_layer: F) -> Out {
        self.0.fold_recursive(collapse_layer)
    }
}