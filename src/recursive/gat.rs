use std::{marker::PhantomData, sync::Arc};

use super::Collapse;
use crate::map_layer::MapLayer;
use futures::{future::BoxFuture, Future, FutureExt};

// TODO: rename to _just_ functor
pub trait Functor {
    type Layer<X>;

    fn map_associated_layer<F, A, B>(input: Self::Layer<A>, f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B;
}

pub enum PartiallyApplied {}

// used to represent partial expansion
impl Functor for Option<PartiallyApplied> {
    type Layer<X> = Option<X>;

    fn map_associated_layer<F, A, B>(input: Self::Layer<A>, mut f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B,
    {
        input.map(f)
    }
}

pub trait JoinFuture: Functor {
    fn join_layer<A>(
        input: Self::Layer<BoxFuture<'static, A>>,
    ) -> BoxFuture<'static, Self::Layer<A>>;
}

pub fn expand_and_collapse_async<Seed, Out, F: JoinFuture>(
    seed: Seed,
    expand_layer: Arc<dyn Fn(Seed) -> BoxFuture<'static, F::Layer<Seed>> + Send + Sync + 'static>,
    collapse_layer: Arc<dyn Fn(F::Layer<Out>) -> BoxFuture<'static, Out> + Send + Sync + 'static>,
) -> impl Future<Output = Out> + Send
where
    F: 'static,
    Seed: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    F::Layer<Seed>: Send + Sync + 'static,
    F::Layer<Out>: Send + Sync + 'static,
    for<'a> F::Layer<BoxFuture<'a, Out>>: Send + Sync,
    F::Layer<BoxFuture<'static, Out>>: 'static,
{
    async move {
        let layer: F::Layer<Seed> = expand_layer(seed).await;

        let expanded: F::Layer<BoxFuture<'static, Out>> = F::map_associated_layer(layer, |x| {
            expand_and_collapse_async::<Seed, Out, F>(
                x,
                expand_layer.clone(),
                collapse_layer.clone(),
            )
            .boxed()
        });

        let expanded_joined: F::Layer<Out> = F::join_layer(expanded).await;

        let res = collapse_layer(expanded_joined).await;

        res
    }
}

struct MapLayerFromFunctor<Layer, Unwrapped, F: Functor>(
    Layer,
    PhantomData<Unwrapped>,
    PhantomData<F>,
);

impl<F: Functor, A, B> MapLayer<B> for MapLayerFromFunctor<F::Layer<A>, A, F> {
    type Unwrapped = A;

    type To = F::Layer<B>;

    fn map_layer<FF: FnMut(Self::Unwrapped) -> B>(self, f: FF) -> Self::To {
        F::map_associated_layer(self.0, f)
    }
}

impl<L, U, F: Functor> MapLayerFromFunctor<L, U, F> {
    pub fn new(x: L) -> Self {
        MapLayerFromFunctor(x, PhantomData, PhantomData)
    }
}

// TODO: should probably move this elsewhere mb
pub trait Recursive
where
    Self: Sized,
{
    type FunctorToken: Functor;

    fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self>;
}

// struct PartiallyApplied<R: Recursive>{
//     wrapped: R,

// }

pub trait RecursiveExt: Recursive {
    fn fold_recursive<
        Out,
        F: FnMut(<<Self as Recursive>::FunctorToken as Functor>::Layer<Out>) -> Out,
    >(
        self,
        collapse_layer: F,
    ) -> Out;

    fn expand_and_collapse<Seed, Out>(
        seed: Seed,
        expand_layer: impl FnMut(Seed) -> <<Self as Recursive>::FunctorToken as Functor>::Layer<Seed>,
        collapse_layer: impl FnMut(<<Self as Recursive>::FunctorToken as Functor>::Layer<Out>) -> Out,
    ) -> Out;
}

impl<X> RecursiveExt for X
where
    X: Recursive,
{
    fn fold_recursive<
        Out,
        F: FnMut(<<X as Recursive>::FunctorToken as Functor>::Layer<Out>) -> Out,
    >(
        self,
        collapse_layer: F,
    ) -> Out {
        Self::expand_and_collapse(self, Self::into_layer, collapse_layer)
    }

    fn expand_and_collapse<Seed, Out>(
        seed: Seed,
        mut expand_layer: impl FnMut(Seed) -> <<X as Recursive>::FunctorToken as Functor>::Layer<Seed>,
        mut collapse_layer: impl FnMut(<<X as Recursive>::FunctorToken as Functor>::Layer<Out>) -> Out,
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
                    let node =
                        Self::FunctorToken::map_associated_layer(node, |seed| seeds.push(seed));

                    stack.push(State::Collapse(node));
                    stack.extend(seeds.into_iter().map(State::Expand));
                }
                State::Collapse(node) => {
                    let node =
                        Self::FunctorToken::map_associated_layer(node, |_: ()| vals.pop().unwrap());
                    vals.push(collapse_layer(node))
                }
            };
        }
        vals.pop().unwrap()
    }
}

struct CollapseViaRecursive<X>(X);

impl<Out, R: RecursiveExt> Collapse<Out, <<R as Recursive>::FunctorToken as Functor>::Layer<Out>>
    for CollapseViaRecursive<R>
{
    fn collapse_layers<F: FnMut(<<R as Recursive>::FunctorToken as Functor>::Layer<Out>) -> Out>(
        self,
        collapse_layer: F,
    ) -> Out {
        self.0.fold_recursive(collapse_layer)
    }
}
