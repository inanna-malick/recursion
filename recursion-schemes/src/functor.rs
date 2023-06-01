#[cfg(feature = "backcompat")]
use recursion::map_layer::MapLayer;
#[cfg(feature = "backcompat")]
use std::marker::PhantomData;

pub trait Functor
{
    type Layer<X>;

    fn fmap<A, B>(input: Self::Layer<A>, f: impl FnMut(A) -> B) -> Self::Layer<B>;
}

pub trait RefFunctor
{
    type Layer<'a, X>
    where
        X: 'a;

    fn fmap<'a, A, B>(input: Self::Layer<'a, A>, f: impl FnMut(A) -> B) -> Self::Layer<'a, B>;
}

pub trait AsRefF: Functor {
    type RefFunctor<'a>: Functor;

    fn as_ref<'a, A>(
        input: &'a <Self as Functor>::Layer<A>,
    ) -> <Self::RefFunctor<'a> as Functor>::Layer<&'a A>;
}

pub trait ToOwnedF: Functor {
    type OwnedFunctor: Functor;

    fn to_owned<A>(input: <Self as Functor>::Layer<A>)
        -> <Self::OwnedFunctor as Functor>::Layer<A>;
}

pub trait TraverseResult {
    type Layer<X>;

    fn flatten<A, E>(input: Self::Layer<Result<A, E>>) -> Result<Self::Layer<A>, E>;
}

pub struct Compose<F1, F2>(std::marker::PhantomData<F1>, std::marker::PhantomData<F2>);

impl<F1: Functor, F2: Functor> Functor for Compose<F1, F2> {
    type Layer<X> = F1::Layer<F2::Layer<X>>;

    fn fmap<A, B>(input: Self::Layer<A>, mut f: impl FnMut(A) -> B) -> Self::Layer<B>
    {
        #[allow(clippy::redundant_closure)] // this lint is wrong here
        F1::fmap(input, move |x| F2::fmap(x, |x| f(x)))
    }
}

#[derive(Debug)]
pub enum PartiallyApplied {}

// used to represent partial expansion
impl Functor for Option<PartiallyApplied> {
    type Layer<X> = Option<X>;

    fn fmap<A, B>(input: Self::Layer<A>, mut f: impl FnMut(A) -> B) -> Self::Layer<B>
    {
        input.map(f)
    }
}

// used to represent partial expansion
impl<Fst> Functor for (Fst, PartiallyApplied) {
    type Layer<X> = (Fst, X);

    fn fmap<A, B>(input: Self::Layer<A>, mut f: impl FnMut(A) -> B) -> Self::Layer<B>
    {
        (input.0, f(input.1))
    }
}

pub struct PairFunctor;

pub type Paired<F> = Compose<PairFunctor, F>;

impl Functor for PairFunctor {
    type Layer<X> = (X, X);

    fn fmap<A, B>(input: Self::Layer<A>, mut f: impl FnMut(A) -> B) -> Self::Layer<B>
    {
        (f(input.0), f(input.1))
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
