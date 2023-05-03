use genawaiter::{Coroutine, sync::GenBoxed};
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

// the typeclass hierarchy here is a mess but this is ok for now
// basic idea is that this is for working with cases where we clone the fmap'd-over structure while only borrowing the things inside it? yeah
// sort of similar to Option::as_ref()
pub trait FunctorRef: Functor {
    // fn fmap_ref<F, A, B>(input: &<Self as Functor>::Layer<A>, f: F) -> <Self as Functor>::Layer<B>
    // where
    //     F: FnMut(&A) -> B;

    fn as_ref<A>(input: &<Self as Functor>::Layer<A>) -> <Self as Functor>::Layer<&A>;
}

pub trait AsRefF: Functor {
    type RefFunctor<'a>: Functor;

    fn as_ref<'a, A>(
        input: &'a <Self as Functor>::Layer<A>,
    ) -> <Self::RefFunctor<'a> as Functor>::Layer<&'a A>;
}

pub trait EqF: AsRefF {
    fn pair_if_eq<'a, Next>(
        a: &'a <Self as Functor>::Layer<Next>,
        b: &'a <Self as Functor>::Layer<Next>,
    ) -> Option<<Compose<<Self as AsRefF>::RefFunctor<'a>, PairFunctor> as Functor>::Layer<&'a Next>>;
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

    fn fmap<F, A, B>(input: Self::Layer<A>, mut f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B,
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



pub struct PairFunctor;

pub type Paired<F> = Compose<PairFunctor, F>;

impl Functor for PairFunctor {
    type Layer<X> = (X, X);

    fn fmap<F, A, B>(input: Self::Layer<A>, mut f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B,
    {
        (f(input.0), f(input.1))
    }
}

// pub trait FunctorRefExt2: AsRefF {
//     fn expand_and_collapse_ref<'a, Seed, Out>(
//         seed: Seed,
//         expand_layer: impl FnMut(&'a Seed) -> &'a <Self as AsRefF>::RefLayer<Seed>,
//         collapse_layer: impl FnMut(<Self as Functor>::Layer<Out>) -> Out,
//     ) -> Out
//     where
//         <Self as Functor>::Layer<Seed>: 'a,
//         Seed: 'a,
//         Out: 'a;
// }

pub trait FunctorRefExt: FunctorRef {
    fn expand_and_collapse_ref<'a, Seed, Out>(
        seed: &'a Seed,
        expand_layer: impl FnMut(&'a Seed) -> &'a <Self as Functor>::Layer<Seed>,
        collapse_layer: impl FnMut(<Self as Functor>::Layer<Out>) -> Out,
    ) -> Out
    where
        <Self as Functor>::Layer<Seed>: 'a;
}

impl<X> FunctorRefExt for X
where
    X: FunctorRef,
{
    fn expand_and_collapse_ref<'a, Seed, Out>(
        seed: &'a Seed,
        mut expand_layer: impl FnMut(&'a Seed) -> &'a <X as Functor>::Layer<Seed>,
        mut collapse_layer: impl FnMut(<X as Functor>::Layer<Out>) -> Out,
    ) -> Out
    where
        <X as Functor>::Layer<Seed>: 'a,
    {
        enum State<Seed, CollapsableInternal> {
            Expand(Seed),
            Collapse(CollapsableInternal),
        }

        let mut vals: Vec<Out> = vec![];
        let mut stack: Vec<State<_, _>> = vec![State::Expand(seed)];

        while let Some(item) = stack.pop() {
            match item {
                State::Expand(seed) => {
                    let node = expand_layer(seed);
                    let mut seeds: Vec<&Seed> = Vec::new();
                    let node = Self::fmap(Self::as_ref(node), |seed| seeds.push(seed));

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

// opaque token type
#[derive(Debug, Clone)]
pub struct ChildToken<Seed>(Seed);

// NOTE: generator execution _requires_ some sort of resume_with call
//       including first invocation where there is no value to resume with!
// so we just use an Option and panic if it's none when not expected
// struct CoroutineWrapper<Out>{
//     wrapped: Box<dyn Coroutine<Return = Out, Yield = ChildToken, Resume = Out>>,
//     is_first_invocation: bool,
// }

// impl<Out> CoroutineWrapper<Out> {
//     fn new(wrapped: Box<dyn Coroutine<Return = Out, Yield = ChildToken, Resume = Out>>) -> Self {
//         Self {
//             wrapped, is_first_invocation = true,
//         }
//     }
// }

// impl Coroutine for CoroutineWrapper<>

pub fn expand_and_collapse_lazy<X: Functor, Seed: Clone, Out>(
    seed: Seed,
    mut expand_layer: impl FnMut(Seed) -> <X as Functor>::Layer<Seed>,
    // NOTE: 'Out' must be returned as option b/c of ignored initial value, so, idk, w/e
    mut collapse_layer: impl FnMut(X::Layer<ChildToken<Seed>>) -> GenBoxed<ChildToken<Seed>, Option<Out>, Out>,
) -> Out {
    use genawaiter::Coroutine;

    enum State<Seed, X: Functor, Out> {
        Expand(Seed),
        Generator(GenBoxed<ChildToken<Seed>, Option<Out>, Out>),
        Collapse(X::Layer<ChildToken<Seed>>),
    }

    let mut vals: Vec<Out> = vec![];
    let mut stack: Vec<State<Seed, X, Out>> = vec![State::Expand(seed)];

    while let Some(item) = stack.pop() {
        match item {
            State::Expand(seed) => {
                let node = expand_layer(seed);
                // let mut seeds = Vec::new();
                let node = X::fmap(node, |seed| ChildToken(seed));

                stack.push(State::Collapse(node));
                // stack.extend(seeds.into_iter().map(State::Expand));
            }
            State::Collapse(node) => {
                let mut gen = collapse_layer(node);
                match gen.resume_with(None) {
                    genawaiter::GeneratorState::Yielded(child_token) => {
                        stack.push(State::Generator(gen));
                        stack.push(State::Expand(child_token.0.clone()));
                    },
                    genawaiter::GeneratorState::Complete(result) => {
                        vals.push(result);
                    },
                }
                // stack.push(State::Generator(gen));
            }
            State::Generator(mut gen) => {
                let next = vals.pop().expect("must exist or programmer error (I think?)");
                match gen.resume_with(Some(next)) {
                    genawaiter::GeneratorState::Yielded(child_token) => {
                        stack.push(State::Generator(gen));
                        stack.push(State::Expand(child_token.0.clone()));
                    },
                    genawaiter::GeneratorState::Complete(result) => {
                        vals.push(result);
                    },
                }

            }
        };
    }
    vals.pop().unwrap()
}

#[cfg(test)]
mod test {
    use genawaiter::{sync::gen, *};

    #[test]
    fn mytest() {
        let mut generator = gen!({
            let x = yield_!(10);
            format!("x: {:?}", x)
        });
        assert_eq!(generator.resume_with(1), GeneratorState::Yielded(10));
        assert_eq!(
            generator.resume_with(2),
            GeneratorState::Complete("x: 2".to_owned())
        );
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
