use std::{collections::HashMap, sync::Arc};

use crate::frame::MappableFrame;

use futures::{
    future::{BoxFuture, LocalBoxFuture},
    stream::{futures_unordered, FuturesUnordered},
    Future, FutureExt, StreamExt, TryFutureExt,
};
use tokio::sync::{mpsc, oneshot};

pub mod compose;

// mostly just used for Compact (defined over frame, needs to collapse_ref via ref frame)
pub trait MappableFrameRef: MappableFrame {
    type RefFrameToken<'a>: MappableFrame;

    fn as_ref<X>(input: &Self::Frame<X>) -> <Self::RefFrameToken<'_> as MappableFrame>::Frame<&X>;
}

pub trait TryMappableFrame: MappableFrame {
    // NOTE: can I do anything about implicit ordering requirement here?
    fn try_map_frame<A, B, E>(
        input: Self::Frame<A>,
        f: impl FnMut(A) -> Result<B, E>,
    ) -> Result<Self::Frame<B>, E>;
}

pub fn try_expand_and_collapse<F: TryMappableFrame, Seed, Out, E>(
    seed: Seed,
    mut expand_frame: impl FnMut(Seed) -> Result<F::Frame<Seed>, E>,
    mut collapse_frame: impl FnMut(F::Frame<Out>) -> Result<Out, E>,
) -> Result<Out, E> {
    enum State<Seed, CollapsableInternal> {
        Expand(Seed),
        Collapse(CollapsableInternal),
    }

    let mut vals: Vec<Out> = vec![];
    let mut stack = vec![State::Expand(seed)];

    while let Some(item) = stack.pop() {
        match item {
            State::Expand(seed) => {
                let node = expand_frame(seed)?;
                let mut seeds = Vec::new();
                let node = F::map_frame(node, |seed| seeds.push(seed));

                stack.push(State::Collapse(node));
                stack.extend(seeds.into_iter().map(State::Expand));
            }
            State::Collapse(node) => {
                let node = F::map_frame(node, |_: ()| vals.pop().unwrap());
                vals.push(collapse_frame(node)?)
            }
        };
    }
    Ok(vals.pop().unwrap())
}

pub async fn expand_and_collapse_async_new<'a, Seed, Out, E, F>(
    seed: Seed,
    expand_frame: impl Fn(Seed) -> BoxFuture<'a, Result<<F as MappableFrame>::Frame<Seed>, E>>
        + Send
        + Sync
        + 'a,
    collapse_frame: impl Fn(<F as MappableFrame>::Frame<Out>) -> BoxFuture<'a, Result<Out, E>>
        + Send
        + Sync
        + 'a,
) -> Result<Out, E>
where
    E: Send + Sync + 'a,
    F: AsyncMappableFrame + 'a,
    Seed: Send + Sync + 'a,
    Out: Send + Sync + 'a,
    // can be avoided via boxed_local but then the resulting future can't be boxed..
    // PROBLEM: the following thingies leak tf out of implementation-local data (eg error b/c local future)
    // NOTE: we can just have a distinct mappable frame type with Send bounds baked in, that'd solve this
    <F as MappableFrame>::Frame<Out>: Send + 'a,
    <F as MappableFrame>::Frame<oneshot::Receiver<Out>>: Send + 'a,
{
    // TODO: mb hashmap in which all frames are stored instead of in the state enum? might allow for avoiding send constraint on Frame<...>
    //       b/c then nothing ever actually goes into the work pool? but it would still need to get into the hashmap somehow, so idk if that would work

    let mut work_pool: FuturesUnordered<BoxFuture<'a, Result<Vec<State<F, Seed, Out>>, E>>> =
        FuturesUnordered::new();

    let (sender, receiver) = oneshot::channel();

    work_pool.push(async { Ok(vec![State::Expand(sender, seed)]) }.boxed());

    let expand_frame = Arc::new(expand_frame);
    let collapse_frame = Arc::new(collapse_frame);

    while let Some(items) = work_pool.next().await {
        for item in items?.into_iter() {
            let expand_frame = expand_frame.clone();
            let collapse_frame = collapse_frame.clone();
            work_pool.push(item.step(expand_frame, collapse_frame).boxed())
        }
    }

    Ok(receiver.await.unwrap()) // will always terminate
}

pub async fn expand_and_collapse_async_new_2<'a, Seed, Out, E, F>(
    seed: Seed,
    expand_frame: impl Fn(Seed) -> BoxFuture<'a, Result<<F as MappableFrame>::Frame<Seed>, E>>
        + Send
        + Sync
        + 'a,
    collapse_frame: impl Fn(<F as MappableFrame>::Frame<Out>) -> BoxFuture<'a, Result<Out, E>>
        + Send
        + Sync
        + 'a,
) -> Result<Out, E>
where
    E: Send + Sync + 'a,
    F: AsyncMappableFrame + 'a,
    Seed: Send + Sync + 'a,
    Out: Send + Sync + 'a,
    <F as MappableFrame>::Frame<Seed>: Send + Sync + 'static,
    <F as MappableFrame>::Frame<Out>: Send + Sync + 'static,

    // can be avoided via boxed_local but then the resulting future can't be boxed..
    // PROBLEM: the following thingies leak tf out of implementation-local data (eg error b/c local future)
    // NOTE: we can just have a distinct mappable frame type with Send bounds baked in, that'd solve this
    // <F as MappableFrame>::Frame<Out>: Send + 'a,
    // <F as MappableFrame>::Frame<oneshot::Receiver<Out>>: Send + 'a,
{
    // TODO: mb hashmap in which all frames are stored instead of in the state enum? might allow for avoiding send constraint on Frame<...>
    //       b/c then nothing ever actually goes into the work pool? but it would still need to get into the hashmap somehow, so idk if that would work

    let mut work_pool: FuturesUnordered<BoxFuture<'a, Result<(), E>>> =
        FuturesUnordered::new();

    let (sender, receiver) = oneshot::channel();
    let (work_sender, mut work_receiver) = mpsc::channel(1024); // idk what size is right here

    let expand_frame = Arc::new(expand_frame);
    let collapse_frame = Arc::new(collapse_frame);

    let root_item = Step{ seed, sender, work_pool: work_sender};
    work_pool.push(root_item.step::<'a, F, E>(expand_frame.clone(), collapse_frame.clone()).boxed());

    loop {
        tokio::select! {
            // enqueue more work if we have it
            Some(work) = work_receiver.recv() => work_pool.push(work.step::<F, E>(expand_frame.clone(), collapse_frame.clone()).boxed()),
            // push existing work to completion and short circuit if err
            Some(completion) = work_pool.next() => match completion{
                                Ok(_) => continue,
                                Err(e) => return Err(e),
                            },
            else => break, //
        }
    }

    Ok(receiver.await.unwrap()) // will always terminate
}

enum State<F: MappableFrame, Seed, Out> {
    Expand(oneshot::Sender<Out>, Seed),
    Collapse(oneshot::Sender<Out>, F::Frame<oneshot::Receiver<Out>>),
}

impl<F: AsyncMappableFrame, Seed: Sync + Send, Out: Sync + Send> State<F, Seed, Out> {
    async fn step<'a, E: Send + Sync + 'a>(
        self,
        expand_frame: Arc<
            dyn Fn(Seed) -> BoxFuture<'a, Result<<F as MappableFrame>::Frame<Seed>, E>>
                + Send
                + Sync
                + 'a,
        >,
        collapse_frame: Arc<
            dyn Fn(<F as MappableFrame>::Frame<Out>) -> BoxFuture<'a, Result<Out, E>>
                + Send
                + Sync
                + 'a,
        >,
    ) -> Result<Vec<Self>, E> {
        match self {
            State::Expand(sender, seed) => {
                let node = expand_frame(seed).await?;
                let mut seeds = Vec::new();
                let node = F::map_frame(node, |seed| {
                    let (sender, receiver) = oneshot::channel();

                    seeds.push((sender, seed));

                    receiver
                });

                let mut ops = vec![State::Collapse(sender, node)];
                ops.extend(
                    seeds
                        .into_iter()
                        .map(|(sender, node)| State::Expand(sender, node)),
                );

                Ok(ops)
            }
            State::Collapse(sender, node) => {
                let node = F::map_frame_async(node, |receiver| async { receiver.await }.boxed())
                    .await
                    .expect("unexpected oneshot recv error");

                let collapsed = collapse_frame(node).await?;

                sender.send(collapsed).ok().expect("oneshot send failure");

                Ok(Vec::new())
            }
        }
    }
}

struct Step<Seed, Out> {
    seed: Seed,
    sender: oneshot::Sender<Out>,
    work_pool: mpsc::Sender<Self>,
}

impl<Seed: Sync + Send, Out: Sync + Send> Step<Seed, Out> {
    async fn step<'a, F: AsyncMappableFrame, E: Send + Sync + 'a>(
        self,
        expand_frame: Arc<
            dyn Fn(Seed) -> BoxFuture<'a, Result<<F as MappableFrame>::Frame<Seed>, E>>
                + Send
                + Sync
                + 'a,
        >,
        collapse_frame: Arc<
            dyn Fn(<F as MappableFrame>::Frame<Out>) -> BoxFuture<'a, Result<Out, E>>
                + Send
                + Sync
                + 'a,
        >,
    ) -> Result<(), E> {
        let node = expand_frame(self.seed).await?;

        let node = F::map_frame_async(node, |seed| {
            async {
                let (sender, receiver) = oneshot::channel();

                self.work_pool
                    .send(Step {
                        sender,
                        seed,
                        work_pool: self.work_pool.clone(),
                    })
                    .await
                    .ok()
                    .expect("mpsc error");

                let recvd = receiver.await.expect("oneshot recv error");

                Ok(recvd)
            }
            .boxed()
        })
        .await?;

        let collapsed = collapse_frame(node).await?;

        self.sender
            .send(collapsed)
            .ok()
            .expect("oneshot send failure");

        Ok(())
    }
}

pub trait AsyncMappableFrame: MappableFrame {
    // NOTE: what does having 'a here mean/imply? should 'a bound be on A/B/E?
    fn map_frame_async<'a, A, B, E>(
        input: Self::Frame<A>,
        f: impl Fn(A) -> BoxFuture<'a, Result<B, E>> + Send + Sync + 'a,
    ) -> BoxFuture<'a, Result<Self::Frame<B>, E>>
    where
        E: Send + 'a,
        A: Send + 'a,
        B: Send + 'a;
}

pub fn expand_and_collapse_async<Seed, Out, E, F: AsyncMappableFrame>(
    seed: Seed,
    expand_layer: Arc<
        dyn Fn(Seed) -> BoxFuture<'static, Result<<F as MappableFrame>::Frame<Seed>, E>>
            + Send
            + Sync
            + 'static,
    >,
    collapse_layer: Arc<
        dyn Fn(<F as MappableFrame>::Frame<Out>) -> BoxFuture<'static, Result<Out, E>>
            + Send
            + Sync
            + 'static,
    >,
) -> BoxFuture<'static, Result<Out, E>>
where
    F: 'static,
    Seed: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    <F as MappableFrame>::Frame<Seed>: Send + Sync + 'static,
    <F as MappableFrame>::Frame<Out>: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    let expand_layer1 = expand_layer.clone();
    let collapse_layer1 = collapse_layer.clone();

    let (send, recieve) = oneshot::channel();

    async move {
        expand_and_collapse_async_worker::<Seed, Out, E, F>(
            seed,
            expand_layer1.clone(),
            collapse_layer1.clone(),
            send,
        )
        .await;

        recieve.await.unwrap()
    }
    .boxed()
}

// TODO: write as async instead of inline then when compiler isses are ironed out
fn expand_and_collapse_async_worker<Seed, Out, E, F: AsyncMappableFrame>(
    seed: Seed,
    expand_layer: Arc<
        dyn Fn(Seed) -> BoxFuture<'static, Result<<F as MappableFrame>::Frame<Seed>, E>>
            + Send
            + Sync
            + 'static,
    >,
    collapse_layer: Arc<
        dyn Fn(<F as MappableFrame>::Frame<Out>) -> BoxFuture<'static, Result<Out, E>>
            + Send
            + Sync
            + 'static,
    >,
    resp_channel: oneshot::Sender<Result<Out, E>>,
) -> BoxFuture<'static, ()>
where
    F: 'static,
    Seed: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    <F as MappableFrame>::Frame<Seed>: Send + Sync + 'static,
    <F as MappableFrame>::Frame<Out>: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    tokio::spawn(
        async {
            let to_send = expand_and_collapse_async_worker_worker::<Seed, Out, E, F>(
                seed,
                expand_layer,
                collapse_layer,
            )
            .await;

            resp_channel
                .send(to_send)
                .ok()
                .expect("failed to send via oneshot plumbing")
        }
        .boxed(),
    )
    .map(|res| res.expect("join failed?"))
    .boxed()
}

async fn expand_and_collapse_async_worker_worker<Seed, Out, E, F: AsyncMappableFrame>(
    seed: Seed,
    expand_layer: Arc<
        dyn Fn(Seed) -> BoxFuture<'static, Result<<F as MappableFrame>::Frame<Seed>, E>>
            + Send
            + Sync
            + 'static,
    >,
    collapse_layer: Arc<
        dyn Fn(<F as MappableFrame>::Frame<Out>) -> BoxFuture<'static, Result<Out, E>>
            + Send
            + Sync
            + 'static,
    >,
) -> Result<Out, E>
where
    F: 'static,
    Seed: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    <F as MappableFrame>::Frame<Seed>: Send + Sync + 'static,
    <F as MappableFrame>::Frame<Out>: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    let expand_layer1 = expand_layer.clone();
    let collapse_layer1 = collapse_layer.clone();

    let expanded = expand_layer(seed).await?;
    let expanded = F::map_frame_async(expanded, move |x| {
        let (send, recieve) = oneshot::channel();

        expand_and_collapse_async_worker::<Seed, Out, E, F>(
            x,
            expand_layer1.clone(),
            collapse_layer1.clone(),
            send,
        )
        .then(|()| recieve)
        .map(|x| x.expect("receive failed (dropped?)"))
        .boxed()
    })
    .await?;

    let collapsed = collapse_layer(expanded).await?;

    Ok(collapsed)
}
