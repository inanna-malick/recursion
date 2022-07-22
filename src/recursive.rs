use futures::future::BoxFuture;

/// Support for collapsing a structure into a single value, one layer at a time
pub trait Collapse<A, O> {
    fn collapse_layers<F: FnMut(O) -> A>(self, collapse_layer: F) -> A;
}


/// Support for expanding a structure from a seed value, one layer at a time
pub trait Expand<A, O> {
    fn expand_layers<F: Fn(A) -> O>(a: A, expand_layer: F) -> Self;
}


/// Support for asynchronously expanding a structure from a seed value, one layer at a time.
pub trait ExpandAsync<A, O> { 
    fn expand_layers_async<
        'a,
        E: Send + 'a,
        F: Fn(A) -> BoxFuture<'a, Result<O, E>> + Send + Sync + 'a,
    >(
        a: A,
        expand_layer: F,
    ) -> BoxFuture<'a, Result<Self, E>>
    where
        Self: Sized,
        A: Send + 'a;
}
