use futures::{Future, FutureExt};
// use cseedmap::CSeedMap;
use futures::future::BoxFuture;
use std::fmt::Debug;
use std::sync::Arc;
use tokio;
use tokio::sync::{oneshot, watch};

use crate::functor::Functor;

// TODO: cache results later, it's hard and it's not on the immediate path.
// or? what if users just make their own cache lol that'd be nice
// or - dumb version, but just cache yourself, at the level below db-get, eg,
// db access object that dedups gets for immutable objects. TLDR: not my problem

// pub async fn unfold_and_fold_result<
//     'a,
// F, a type parameter of kind * -> * that cannot be represented in rust
//     E: Debug + Send,
//     Seed: Send,
//     Out: Send,
//     GenerateExpr: Functor<(), Unwrapped = Seed, To = U>, // F<Seed>
//     ConsumeExpr,                                         // F<Out>
//     U: Functor<Out, To = ConsumeExpr, Unwrapped = ()>,   // F<U>
//     Alg: FnMut(ConsumeExpr) -> BoxFuture<'a, Result<Out, E>>, // F<Out> -> Result<Out, E>
//     CoAlg: Fn(Seed) -> BoxFuture<'a, Result<GenerateExpr, E>>, // Seed -> Result<F<Seed>, E>
// >(
//     seed: Seed,
//     coalg: CoAlg, // Seed -> F<Seed>
//     mut alg: Alg, // F<Out> -> Out
// ) -> Result<Out, E> {
//     let (send, receive) = oneshot::channel(); // randomly chose this channel buffer size..
//                                               // let memoizer = Arc::new(CSeedMap::new());

//     unfold_and_fold_result_helper(send, seed, coalg, alg);

//     // unwrap is just for error on the channel, idk - handle this somehow mb?
//     receive.await.unwrap()
// }

fn unfold_and_fold_result_helper<
    // F, a type parameter of kind * -> * that cannot be represented in rust
    E: Debug + Send + 'static,
    Seed: Send + 'static,
    Out: Send + 'static,
    GenerateExpr: Functor<oneshot::Receiver<Result<Out, E>>, Unwrapped = Seed, To = U> + Send, // F<Seed>
    ConsumeExpr: Send,                                                   // F<Out>
    U: AsyncFlatten<'static, To = ConsumeExpr, Err = E> + Send, // F<BoxFuture<Result<Out, E>>
    AlgRes: Future<Output = Result<Out, E>> + Send + Sync,
    Alg: Fn(ConsumeExpr) -> AlgRes + Send + Sync + 'static, // F<Out> -> Result<Out, E>
    CoAlgRes: Future<Output = Result<GenerateExpr, E>> + Send + Sync,
    CoAlg: Fn(Seed) -> CoAlgRes + Send + Sync + 'static, // Seed -> Result<F<Seed>, E>
>(
    resp_chan: oneshot::Sender<Result<Out, E>>,
    seed: Seed,
    coalg: Arc<CoAlg>, // Seed -> F<Seed>
    alg: Arc<Alg>,     // F<Out> -> Out
) {
    let jh = tokio::spawn(async move {
        let res = unfold_and_fold_result_worker(seed, coalg, alg).await;
        resp_chan.send(res).ok().unwrap()
    });
}

async fn unfold_and_fold_result_worker<
    // F, a type parameter of kind * -> * that cannot be represented in rust
    E: Debug + Send + 'static,
    Seed: Send + 'static,
    Out: Send + 'static,
    GenerateExpr: Functor<oneshot::Receiver<Result<Out, E>>, Unwrapped = Seed, To = U> + Send, // F<Seed>
    ConsumeExpr: Send,                                     // F<Out>
    U: AsyncFlatten<'static, To = ConsumeExpr, Err = E> + Send, // F<BoxFuture<Result<Out, E>>
    AlgRes: Future<Output = Result<Out, E>> + Send + Sync,
    Alg: Fn(ConsumeExpr) -> AlgRes + Send + Sync + 'static, // F<Out> -> Result<Out, E>
    CoAlgRes: Future<Output = Result<GenerateExpr, E>> + Send + Sync,
    CoAlg: Fn(Seed) -> CoAlgRes + Send + Sync + 'static, // Seed -> Result<F<Seed>, E>
>(
    seed: Seed,
    coalg: Arc<CoAlg>, // Seed -> F<Seed>
    alg: Arc<Alg>,     // F<Out> -> Out
) -> Result<Out, E> {
    let layer = coalg(seed).await?;
    let layer = layer.fmap(|seed| {
        let (sender, receiver) = oneshot::channel();
        // TODO: save and accumulate join handles
        // TODO: use oneshots or something here to improve reliability, also it'd be nice to have all join handles to trigger
        //       abort on early failure
        unfold_and_fold_result_helper(sender, seed, coalg.clone(), alg.clone());

        receiver
    });
    let layer = layer.flatten_async().await?;
    alg(layer).await
}

// impl'd for F<BoxFuture<Result<A, Err>>> where To is BoxFuture<<Result<F<A>, Err>>
pub trait AsyncFlatten<'a> {
    type To;
    type Err;
    // todo: incorporate tryfuture short circuit
    fn flatten_async(self) -> BoxFuture<'a, Result<Self::To, Self::Err>>;
}
