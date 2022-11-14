use std::{marker::PhantomData, sync::Arc};

use super::Collapse;
use crate::map_layer::MapLayer;
use futures::{future::BoxFuture, Future, FutureExt};

// TODO: rename to _just_ functor
pub trait Functor {
    type Layer<X>;

    fn fmap<F, A, B>(input: Self::Layer<A>, f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B;
}

pub struct Compose<F1, F2>(std::marker::PhantomData<F1>, std::marker::PhantomData<F2>);

impl<F1: Functor, F2: Functor> Functor for Compose<F1, F2> {
    type Layer<X> = F1::Layer<F2::Layer<X>>;

    fn fmap<F, A, B>(input: Self::Layer<A>, mut f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B,
    {
        F1::fmap(input, move |x| F2::fmap(x, |x| f(x)))
    }
}

pub enum PartiallyApplied {}

// used to represent partial expansion
impl Functor for Option<PartiallyApplied> {
    type Layer<X> = Option<X>;

    fn fmap<F, A, B>(input: Self::Layer<A>, f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B,
    {
        input.map(f)
    }
}

// used to represent partial expansion
impl<Fst> Functor for (Fst, PartiallyApplied) {
    type Layer<X> = (Fst, X);

    fn fmap<F, A, B>(input: Self::Layer<A>, mut f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B,
    {
        (input.0, f(input.1))
    }
}

// pub trait JoinFuture: Functor {
//     fn join_layer<A>(
//         input: Self::Layer<BoxFuture<'static, A>>,
//     ) -> BoxFuture<'static, Self::Layer<A>>;
// }

// pub fn expand_and_collapse_async<Seed, Out, F: JoinFuture>(
//     seed: Seed,
//     expand_layer: Arc<dyn Fn(Seed) -> BoxFuture<'static, F::Layer<Seed>> + Send + Sync + 'static>,
//     collapse_layer: Arc<dyn Fn(F::Layer<Out>) -> BoxFuture<'static, Out> + Send + Sync + 'static>,
// ) -> impl Future<Output = Out> + Send
// where
//     F: 'static,
//     Seed: Send + Sync + 'static,
//     Out: Send + Sync + 'static,
//     F::Layer<Seed>: Send + Sync + 'static,
//     F::Layer<Out>: Send + Sync + 'static,
//     for<'a> F::Layer<BoxFuture<'a, Out>>: Send + Sync,
//     F::Layer<BoxFuture<'static, Out>>: 'static,
// {
//     async move {
//         let layer: F::Layer<Seed> = expand_layer(seed).await;

//         let expanded: F::Layer<BoxFuture<'static, Out>> = F::fmap(layer, |x| {
//             expand_and_collapse_async::<Seed, Out, F>(
//                 x,
//                 expand_layer.clone(),
//                 collapse_layer.clone(),
//             )
//             .boxed()
//         });

//         let expanded_joined: F::Layer<Out> = F::join_layer(expanded).await;

//         let res = collapse_layer(expanded_joined).await;

//         res
//     }
// }

struct MapLayerFromFunctor<Layer, Unwrapped, F: Functor>(
    Layer,
    PhantomData<Unwrapped>,
    PhantomData<F>,
);

impl<F: Functor, A, B> MapLayer<B> for MapLayerFromFunctor<F::Layer<A>, A, F> {
    type Unwrapped = A;

    type To = F::Layer<B>;

    fn map_layer<FF: FnMut(Self::Unwrapped) -> B>(self, f: FF) -> Self::To {
        F::fmap(self.0, f)
    }
}

impl<L, U, F: Functor> MapLayerFromFunctor<L, U, F> {
    pub fn new(x: L) -> Self {
        MapLayerFromFunctor(x, PhantomData, PhantomData)
    }
}

pub trait Recursive
where
    Self: Sized,
{
    type FunctorToken: Functor;

    fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self>;
}

pub struct Annotated<R: Recursive, A> {
    pub wrapped: R,
    pub f: Arc<
        // TODO: probably doesn't need to be an arc but (shrug emoji)
        dyn Fn(&<<R as Recursive>::FunctorToken as Functor>::Layer<R>) -> A,
    >,
}

impl<R: Recursive, A> Recursive for Annotated<R, A> {
    type FunctorToken = Compose<(A, PartiallyApplied), R::FunctorToken>;

    fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
        let layer = R::into_layer(self.wrapped);
        let layer = ((self.f)(&layer), layer);
        Self::FunctorToken::fmap(layer, move |wrapped| Annotated {
            wrapped,
            f: self.f.clone(),
        })
    }
}



// TODO: futumorphism to allow for partial non-async expansion? yes! but (I think) needs to be erased for collapse phase
// TODO: b/c at that point there's no need for that info..

pub struct WithContext<R: Recursive>(pub R);

impl<R: Recursive + Copy> Recursive for WithContext<R> {
    type FunctorToken = Compose<R::FunctorToken, (R, PartiallyApplied)>;

    fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
        let layer = R::into_layer(self.0);
        R::FunctorToken::fmap(layer, move |wrapped| (wrapped, WithContext(wrapped)))
    }
}

pub struct PartialExpansion<R: Recursive> {
    pub wrapped: R,
    pub f: Arc<
        // TODO: probably doesn't need to be an arc but (shrug emoji)
        dyn Fn(
            <<R as Recursive>::FunctorToken as Functor>::Layer<R>,
        ) -> <<R as Recursive>::FunctorToken as Functor>::Layer<Option<R>>,
    >,
}

impl<R: Recursive> Recursive for PartialExpansion<R> {
    type FunctorToken = Compose<R::FunctorToken, Option<PartiallyApplied>>;

    fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
        let partially_expanded = (self.f)(self.wrapped.into_layer());
        Self::FunctorToken::fmap(partially_expanded, move |wrapped| PartialExpansion {
            wrapped,
            f: self.f.clone(),
        })
    }
}

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
                    let node = Self::FunctorToken::fmap(node, |seed| seeds.push(seed));

                    stack.push(State::Collapse(node));
                    stack.extend(seeds.into_iter().map(State::Expand));
                }
                State::Collapse(node) => {
                    let node = Self::FunctorToken::fmap(node, |_: ()| vals.pop().unwrap());
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
