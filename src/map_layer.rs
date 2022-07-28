/// Provides the ability to map over some structure 'Layer',
/// such that 'Self' is 'Layer<Unwrapped>', via a function 'Fn(Unwrapped) -> B'
/// producing a value 'To' such 'To' is 'Layer<B>'.
///
/// The function provided to map_layer MUST be strictly applied.
pub trait MapLayer<B> {
    // where Self = Layer<A>
    type Unwrapped; // A
    type To; // Layer<B>
    /// Additional constraint not present in haskell, req'd for stack machine eval:
    ///   for any F<A>, F<B>, etc, where all structure but B/A is identical,
    ///   fmap must visit nodes in the same order each time it is called
    ///   given that F<B> is created by mapping some function over F<A>
    ///   note that enforcing this property may be problematic for, Hashmaps/Sets/etc
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To;
}

// basically just From/To but we want something clearly context-specific and, idk, lawful probably
pub trait Project {
    // A
    type To; // F<A>
    fn project(self) -> Self::To;
}

pub trait CoProject {
    // A
    type From; // F<A>
    fn coproject(f: Self::From) -> Self;
}
