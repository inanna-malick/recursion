use crate::frame::MappableFrame;

pub struct Compose<F1, F2>(std::marker::PhantomData<F1>, std::marker::PhantomData<F2>);

impl<F1: MappableFrame, F2: MappableFrame> MappableFrame for Compose<F1, F2> {
    type Frame<X> = F1::Frame<F2::Frame<X>>;

    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
        #[allow(clippy::redundant_closure)] // this lint is wrong here
        F1::map_frame(input, move |x| F2::map_frame(x, |x| f(x)))
    }
}

#[derive(Debug)]
pub enum PartiallyApplied {}

// used to represent partial expansion
impl MappableFrame for Option<PartiallyApplied> {
    type Frame<X> = Option<X>;

    fn map_frame<A, B>(input: Self::Frame<A>, f: impl FnMut(A) -> B) -> Self::Frame<B> {
        input.map(f)
    }
}

// used to represent partial expansion
impl<Fst> MappableFrame for (Fst, PartiallyApplied) {
    type Frame<X> = (Fst, X);

    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
        (input.0, f(input.1))
    }
}

pub struct PairMappableFrame;

pub type Paired<F> = Compose<PairMappableFrame, F>;

impl MappableFrame for PairMappableFrame {
    type Frame<X> = (X, X);

    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
        (f(input.0), f(input.1))
    }
}
