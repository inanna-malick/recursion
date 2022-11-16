use std::sync::Arc;

use futures::{future::BoxFuture, Future, FutureExt};

use crate::functor::Functor;

pub trait JoinFuture: Functor {
    fn join_layer<A>(
        input: Self::Layer<BoxFuture<'static, A>>,
    ) -> BoxFuture<'static, Self::Layer<A>>;
}

pub fn expand_and_collapse_async<Seed, Out, F: JoinFuture>(
    seed: Seed,
    expand_layer: Arc<dyn Fn(Seed) -> BoxFuture<'static, F::Layer<Seed>> + Send + Sync + 'static>,
    collapse_layer: Arc<dyn Fn(F::Layer<Out>) -> BoxFuture<'static, Out> + Send + Sync + 'static>,
) -> impl Future<Output = Out> + Send
where
    F: 'static,
    Seed: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    F::Layer<Seed>: Send + Sync + 'static,
    F::Layer<Out>: Send + Sync + 'static,
    for<'a> F::Layer<BoxFuture<'a, Out>>: Send + Sync,
    F::Layer<BoxFuture<'static, Out>>: 'static,
{
    async move {
        let layer: F::Layer<Seed> = expand_layer(seed).await;

        let expanded: F::Layer<BoxFuture<'static, Out>> = F::fmap(layer, |x| {
            expand_and_collapse_async::<Seed, Out, F>(
                x,
                expand_layer.clone(),
                collapse_layer.clone(),
            )
            .boxed()
        });

        let expanded_joined: F::Layer<Out> = F::join_layer(expanded).await;

        let res = collapse_layer(expanded_joined).await;

        res
    }
}

// so many trait bounds... too many?
pub trait RecursiveAsync
where
    Self: Sized,
{
    type JoinFutureToken: JoinFuture;

    fn into_layer(self) -> BoxFuture<'static, <Self::JoinFutureToken as Functor>::Layer<Self>>;
}

pub trait RecursiveAsyncExt: RecursiveAsync {
    fn fold_recursive<Out: Send + Sync + 'static>(
        self,
        collapse_layer: Arc<
            dyn Fn(
                    <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Out>,
                ) -> BoxFuture<'static, Out>
                + Send
                + Sync
                + 'static,
        >,
    ) -> BoxFuture<'static, Out>
    where
        Self: Send + Sync + 'static,
        <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Self>: Send + Sync,
        <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Out>: Send + Sync,
        for<'a> <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<BoxFuture<'a, Out>>:
            Send + Sync;
}

impl<X> RecursiveAsyncExt for X
where
    X: RecursiveAsync + Send + Sync,
{
    fn fold_recursive<Out: Send + Sync + 'static>(
        self,
        collapse_layer: Arc<
            dyn Fn(
                    <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Out>,
                ) -> BoxFuture<'static, Out>
                + Send
                + Sync
                + 'static,
        >,
    ) -> BoxFuture<'static, Out>
    where
        Self: Send + Sync + 'static,
        <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Self>: Send + Sync,
        <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Out>: Send + Sync,
        for<'a> <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<BoxFuture<'a, Out>>:
            Send + Sync,
    {
        expand_and_collapse_async(self, Arc::new(Self::into_layer), collapse_layer).boxed()
    }
}
