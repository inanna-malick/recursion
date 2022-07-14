use futures::future::BoxFuture;

/// Support for recursion - folding a recursive structure into a single seed
pub trait Recursive<A, O> {
    fn fold<F: FnMut(O) -> A>(self, alg: F) -> A;
}

/// Support for corecursion - unfolding a recursive structure from a seed
pub trait CoRecursive<A, O> {
    fn unfold<F: Fn(A) -> O>(a: A, coalg: F) -> Self;
}

pub trait CoRecursiveAsync<A, O> {
    fn unfold_async<'a, E: Send + 'a, F: Fn(A) -> BoxFuture<'a, Result<O, E>> + Send + Sync + 'a>(
        a: A,
        coalg: F,
    ) -> BoxFuture<'a, Result<Self, E>>
    where
        Self: Sized,
        A: Send + 'a;
}
