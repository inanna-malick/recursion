use futures::future::BoxFuture;

/// Support for recursion - folding a recursive structure into a single seed
pub trait Foldable<A, O> {
    fn fold<F: FnMut(O) -> A>(self, fold_layer: F) -> A;
}

/// Support for corecursion - unfolding a recursive structure from a seed
pub trait Generatable<A, O> {
    fn generate_layer<F: Fn(A) -> O>(a: A, generate_layer: F) -> Self;
}

pub trait GeneratableAsync<A, O> {
    fn unfold_async<'a, E: Send + 'a, F: Fn(A) -> BoxFuture<'a, Result<O, E>> + Send + Sync + 'a>(
        a: A,
        coalg: F,
    ) -> BoxFuture<'a, Result<Self, E>>
    where
        Self: Sized,
        A: Send + 'a;
}
