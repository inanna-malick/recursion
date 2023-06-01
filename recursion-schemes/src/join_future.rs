use std::sync::Arc;

use futures::{future::BoxFuture, FutureExt};

use crate::functor::Functor;
use crate::recursive::Corecursive;
use tokio::sync::oneshot;

pub trait JoinFuture: Functor {
    fn join_layer<A: Send + 'static>(
        input: <Self as Functor>::Layer<BoxFuture<'static, A>>,
    ) -> BoxFuture<'static, <Self as Functor>::Layer<A>>;
}

pub fn expand_and_collapse_async<Seed, Out, F: JoinFuture>(
    seed: Seed,
    expand_layer: Arc<
        dyn Fn(Seed) -> BoxFuture<'static, <F as Functor>::Layer<Seed>> + Send + Sync + 'static,
    >,
    collapse_layer: Arc<
        dyn Fn(<F as Functor>::Layer<Out>) -> BoxFuture<'static, Out> + Send + Sync + 'static,
    >,
) -> BoxFuture<'static, Out>
where
    F: 'static,
    Seed: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    <F as Functor>::Layer<Seed>: Send + Sync + 'static,
    <F as Functor>::Layer<Out>: Send + Sync + 'static,
{
    let expand_layer1 = expand_layer.clone();
    let collapse_layer1 = collapse_layer.clone();

    let (send, recieve) = oneshot::channel();

    expand_and_collapse_async_worker::<Seed, Out, F>(
        seed,
        expand_layer1.clone(),
        collapse_layer1.clone(),
        send,
    )
    .then(|()| recieve.map(|res| res.unwrap()))
    .boxed()
}

// TODO: write as async instead of inline then when compiler isses are ironed out
fn expand_and_collapse_async_worker<Seed, Out, F: JoinFuture>(
    seed: Seed,
    expand_layer: Arc<
        dyn Fn(Seed) -> BoxFuture<'static, <F as Functor>::Layer<Seed>> + Send + Sync + 'static,
    >,
    collapse_layer: Arc<
        dyn Fn(<F as Functor>::Layer<Out>) -> BoxFuture<'static, Out> + Send + Sync + 'static,
    >,
    resp_channel: oneshot::Sender<Out>,
) -> BoxFuture<'static, ()>
where
    F: 'static,
    Seed: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    <F as Functor>::Layer<Seed>: Send + Sync + 'static,
    <F as Functor>::Layer<Out>: Send + Sync + 'static,
{
    let expand_layer1 = expand_layer.clone();
    let collapse_layer1 = collapse_layer.clone();

    tokio::spawn(
        expand_layer(seed)
            .then(move |layer| {
                let expand_layer2 = expand_layer1.clone();
                let collapse_layer2 = collapse_layer1.clone();

                let expanded = F::fmap(layer, |x| {
                    let (send, recieve) = oneshot::channel();

                    expand_and_collapse_async_worker::<Seed, Out, F>(
                        x,
                        expand_layer2.clone(),
                        collapse_layer2.clone(),
                        send,
                    )
                    .then(|()| recieve)
                    .map(|x| x.expect("receive failed (dropped?)"))
                    .boxed()
                });

                F::join_layer(expanded)
                    .then(move |expanded_joined| collapse_layer1(expanded_joined))
                    .map(|res| match resp_channel.send(res) {
                        Ok(res) => res,
                        Err(_) => panic!("send failed (???)"),
                    })
            })
            .boxed(),
    )
    .map(|res| res.expect("join failed?"))
    .boxed()
}

// // so many trait bounds... too many?
// pub trait RecursiveAsync
// where
//     Self: Sized,
// {
//     type JoinFutureToken: JoinFuture;

//     fn into_layer(
//         self,
//     ) -> BoxFuture<'static, <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Self>>;
// }

// pub trait Corecursive: Corecursive
// where
//     Self: Sized,
// {
//     type JoinFutureToken: JoinFuture;

//     fn from_layer(
//         layer: <<Self as Corecursive>::JoinFutureToken as Functor>::Layer<Self>,
//     ) -> BoxFuture<'static, Self>;
// }

pub trait CorecursiveAsyncExt: Corecursive {
    fn unfold_recursive_async<Seed: Send + Sync + 'static>(
        seed: Seed,
        expand_layer: Arc<
            dyn Fn(
                    Seed,
                ) -> BoxFuture<
                    'static,
                    <<Self as Corecursive>::MappableFrame as Functor>::Layer<Seed>,
                > + Send
                + Sync
                + 'static,
        >,
    ) -> BoxFuture<'static, Self>
    where
        <Self as Corecursive>::MappableFrame: JoinFuture,
        Self: Send + Sync + 'static,
        <<Self as Corecursive>::MappableFrame as Functor>::Layer<Seed>: Send + Sync,
        <<Self as Corecursive>::MappableFrame as Functor>::Layer<Self>: Send + Sync;
}

impl<X> CorecursiveAsyncExt for X
where
    X: Corecursive + Send + Sync,
{
    fn unfold_recursive_async<Seed: Send + Sync + 'static>(
        seed: Seed,
        expand_layer: Arc<
            dyn Fn(
                    Seed,
                ) -> BoxFuture<
                    'static,
                    <<Self as Corecursive>::MappableFrame as Functor>::Layer<Seed>,
                > + Send
                + Sync
                + 'static,
        >,
    ) -> BoxFuture<'static, Self>
    where
        <Self as Corecursive>::MappableFrame: JoinFuture,
        Self: Send + Sync + 'static,
        <<Self as Corecursive>::MappableFrame as Functor>::Layer<Seed>: Send + Sync,
        <<Self as Corecursive>::MappableFrame as Functor>::Layer<Self>: Send + Sync,
    {
        expand_and_collapse_async::<Seed, Self, Self::MappableFrame>(
            seed,
            expand_layer,
            Arc::new(|x| futures::future::ready(Self::from_layer(x)).boxed()),
        )
        .boxed()
    }
}

// impl<R: RecursiveAsync> RecursiveAsync for PartialExpansion<R> {
//     type FunctorToken = Compose<R::FunctorToken, Option<PartiallyApplied>>;

//     fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
//         let partially_expanded = (self.f)(self.wrapped.into_layer());
//         Self::FunctorToken::fmap(partially_expanded, move |wrapped| PartialExpansion {
//             wrapped,
//             f: self.f.clone(),
//         })
//     }
// }

// pub trait RecursiveAsyncExt: RecursiveAsync {
//     fn fold_recursive_async<Out: Send + Sync + 'static>(
//         self,
//         collapse_layer: Arc<
//             dyn Fn(
//                     <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Out>,
//                 ) -> BoxFuture<'static, Out>
//                 + Send
//                 + Sync
//                 + 'static,
//         >,
//     ) -> BoxFuture<'static, Out>
//     where
//         <Self as RecursiveAsync>::JoinFutureToken: Functor,
//         Self: Send + Sync + 'static,
//         <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Out>: Send + Sync,
//         <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Self>: Send + Sync;
// }

// impl<X> RecursiveAsyncExt for X
// where
//     X: RecursiveAsync + Send + Sync,
// {
//     fn fold_recursive_async<Out: Send + Sync + 'static>(
//         self,
//         collapse_layer: Arc<
//             dyn Fn(
//                     <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Out>,
//                 ) -> BoxFuture<'static, Out>
//                 + Send
//                 + Sync
//                 + 'static,
//         >,
//     ) -> BoxFuture<'static, Out>
//     where
//         <Self as RecursiveAsync>::JoinFutureToken: Functor,
//         Self: Send + Sync + 'static,
//         <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Out>: Send + Sync,
//         <<Self as RecursiveAsync>::JoinFutureToken as Functor>::Layer<Self>: Send + Sync,
//     {
//         expand_and_collapse_async::<Self, Out, Self::JoinFutureToken>(
//             self,
//             Arc::new(Self::into_layer),
//             collapse_layer,
//         )
//         .boxed()
//     }
// }

// impl<R: RecursiveAsync> RecursiveAsync for PartialExpansion<R> {
//     type FunctorToken = Compose<R::FunctorToken, Option<PartiallyApplied>>;

//     fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
//         let partially_expanded = (self.f)(self.wrapped.into_layer());
//         Self::FunctorToken::fmap(partially_expanded, move |wrapped| PartialExpansion {
//             wrapped,
//             f: self.f.clone(),
//         })
//     }
// }
