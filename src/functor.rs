

/// Provides the ability to map over some structure 'Layer',
/// such that 'Self' is 'Layer<Unwrapped>', via a function 'Fn(Unwrapped) -> B'
/// producing a value 'To' such 'To' is 'Layer<B>'. 
/// 
/// The function provided to fmap MUST be strictly applied.
/// This precludes defining what would otherwise be valid functors for, eg,
/// 'BoxFuture<'_, A>'.
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
