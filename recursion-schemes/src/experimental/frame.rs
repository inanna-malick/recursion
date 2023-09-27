use std::{collections::HashMap, sync::Arc};

use crate::frame::MappableFrame;

use futures::{
    future::BoxFuture,
    stream::{futures_unordered, FuturesUnordered},
    Future, FutureExt, StreamExt, TryFutureExt,
};
use tokio::sync::oneshot;

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

type ExpandIdx = usize;
type CollapseIdx = usize;
// pub struct RunningComputation<F: AsyncMappableFrame, In, Out>{
//     // starts with one element
//     expansions: HashMap<ExpandIdx, In>, // to be expanded
//     expansion_keygen: usize, // increment for next key
//     collapses: HashMap<CollapseIdx, F::Frame<Out>>,
//     collapse_keygen: usize,  // increment for next key
// }

enum State<F: MappableFrame, Seed, Out> {
    Expand(oneshot::Sender<Out>, Seed),
    Collapse(oneshot::Sender<Out>, F::Frame<oneshot::Receiver<Out>>),
}

impl<F: AsyncMappableFrame, Seed: Sync + Send, Out: Sync + Send> State<F, Seed, Out> {
    async fn step<'a>(
        self,
        expand_frame: Arc<
            dyn Fn(Seed) -> BoxFuture<'a, <F as MappableFrame>::Frame<Seed>> + Send + Sync + 'a,
        >,
        collapse_frame: Arc<
            dyn Fn(<F as MappableFrame>::Frame<Out>) -> BoxFuture<'a, Out> + Send + Sync + 'a,
        >,
    ) -> Vec<Self> {
        match self {
            State::Expand(sender, seed) => {
                let node = expand_frame(seed).await;
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

                ops
            }
            State::Collapse(sender, node) => {
                let node = F::map_frame_async(node, |receiver| async { receiver.await }.boxed())
                    .await
                    .expect("unexpected oneshot recv error");

                let collapsed = collapse_frame(node).await;

                sender.send(collapsed).ok().expect("oneshot send failure");

                Vec::new()
            }
        }
    }
}


pub async fn expand_and_collapse_async_new<'a, Seed, Out, F>(
    seed: Seed,
    expand_frame: impl Fn(Seed) -> BoxFuture<'a, <F as MappableFrame>::Frame<Seed>> + Send + Sync + 'a,
    collapse_frame: impl Fn(<F as MappableFrame>::Frame<Out>) -> BoxFuture<'a, Out> + Send + Sync + 'a,
) -> Out
where
    F: AsyncMappableFrame + 'a,
    Seed: Send + Sync + 'a,
    Out: Send + Sync + 'a,
    <F as MappableFrame>::Frame<oneshot::Receiver<Out>>: Send + 'a,
    <F as MappableFrame>::Frame<Out>: Send + 'a,
{
    let mut work_pool: FuturesUnordered<BoxFuture<'a, Vec<State<F, Seed, Out>>>> =
        FuturesUnordered::new();

    let (sender, receiver) = oneshot::channel();

    work_pool.push(async { vec![State::Expand(sender, seed)] }.boxed());

    let expand_frame = Arc::new(expand_frame);
    let collapse_frame = Arc::new(collapse_frame);

    while let Some(items) = work_pool.next().await {
        for item in items.into_iter() {
            let expand_frame = expand_frame.clone();
            let collapse_frame = collapse_frame.clone();
            work_pool.push(
                async move {
                    item.step(expand_frame, collapse_frame)
                        .await
                }
                .boxed(),
            )
        }
    }

    receiver.await.unwrap() // must always terminate, one hopes
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
