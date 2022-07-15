pub trait Functor<B> {
    type Unwrapped;
    type To;
    /// fmap over an owned value
    /// Additional constraint not present in haskell, req'd for stack machine eval:
    ///   for any F<A>, F<B>, etc, where all structure but B/A is identical,
    ///   fmap must visit nodes in the same order each time it is called
    ///   given that F<B> is created by mapping some function over F<A>
    ///   note that enforcing this property may be problematic for, Hashmaps/Sets/etc
    fn fmap<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To;
}
