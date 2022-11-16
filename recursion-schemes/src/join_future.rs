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
