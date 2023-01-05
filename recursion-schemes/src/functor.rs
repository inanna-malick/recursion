#[cfg(feature = "backcompat")]
use recursion::map_layer::MapLayer;
#[cfg(feature = "backcompat")]
use std::marker::PhantomData;

pub trait Functor // where
//     Self: Self::Layer<PartiallyApplied>,
{
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
        #[allow(clippy::redundant_closure)] // this lint is wrong here
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

pub trait FunctorExt: Functor {
    fn expand_and_collapse<Seed, Out>(
        seed: Seed,
        expand_layer: impl FnMut(Seed) -> <Self as Functor>::Layer<Seed>,
        collapse_layer: impl FnMut(<Self as Functor>::Layer<Out>) -> Out,
    ) -> Out;
}

impl<X> FunctorExt for X
where
    X: Functor,
{
    fn expand_and_collapse<Seed, Out>(
        seed: Seed,
        mut expand_layer: impl FnMut(Seed) -> <X as Functor>::Layer<Seed>,
        mut collapse_layer: impl FnMut(<X as Functor>::Layer<Out>) -> Out,
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
                    let node = Self::fmap(node, |seed| seeds.push(seed));

                    stack.push(State::Collapse(node));
                    stack.extend(seeds.into_iter().map(State::Expand));
                }
                State::Collapse(node) => {
                    let node = Self::fmap(node, |_: ()| vals.pop().unwrap());
                    vals.push(collapse_layer(node))
                }
            };
        }
        vals.pop().unwrap()
    }
}

#[cfg(feature = "backcompat")]
pub struct MapLayerFromFunctor<Layer, Unwrapped, F: Functor>(
    Layer,
    PhantomData<Unwrapped>,
    PhantomData<F>,
);

#[cfg(feature = "backcompat")]
impl<F: Functor, A, B> MapLayer<B> for MapLayerFromFunctor<F::Layer<A>, A, F> {
    type Unwrapped = A;

    type To = F::Layer<B>;

    fn map_layer<FF: FnMut(Self::Unwrapped) -> B>(self, f: FF) -> Self::To {
        F::fmap(self.0, f)
    }
}

#[cfg(feature = "backcompat")]
impl<L, U, F: Functor> MapLayerFromFunctor<L, U, F> {
    pub fn new(x: L) -> Self {
        MapLayerFromFunctor(x, PhantomData, PhantomData)
    }
}
