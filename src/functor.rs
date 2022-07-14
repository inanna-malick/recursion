pub trait Functor<B> {
    type Unwrapped;
    type To;
    /// fmap over an owned value
    fn fmap<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To;
}
