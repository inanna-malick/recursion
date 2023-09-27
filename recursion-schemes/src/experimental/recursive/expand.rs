use std::sync::Arc;

use futures::{future::BoxFuture, FutureExt};

use crate::{frame::MappableFrame, recursive::expand::Expandable, experimental::frame::{expand_and_collapse_async, AsyncMappableFrame}};

pub trait ExpandableAsync: Expandable<FrameToken = Self::AsyncFrameToken>
where
    Self: Sized,
{
    type AsyncFrameToken: AsyncMappableFrame;

    /// defined on trait for convenience and to allow for optimized impls
    fn expand_frames_async<In, E>(
        seed: In,
        expand_frame: impl Fn(
                In,
            ) -> BoxFuture<'static, Result<<Self::AsyncFrameToken as MappableFrame>::Frame<In>, E>>
            + Send
            + Sync
            + 'static,
    ) -> BoxFuture<'static, Result<Self, E>>
    where
        Self: Send + Sync + 'static,
        In: Send + Sync + 'static,
        <Self::AsyncFrameToken as MappableFrame>::Frame<Self>: Send + Sync + 'static,
        <Self::AsyncFrameToken as MappableFrame>::Frame<In>: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        expand_and_collapse_async::<In, Self, E, Self::AsyncFrameToken>(
            seed,
            Arc::new(expand_frame),
            Arc::new(|frame| std::future::ready(Ok(Self::from_frame(frame))).boxed()),
        )
    }

}
