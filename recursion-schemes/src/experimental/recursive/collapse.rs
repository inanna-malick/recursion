use std::sync::Arc;

use futures::{future::BoxFuture, Future, FutureExt};

use crate::{
    experimental::frame::*,
    frame::{expand_and_collapse, MappableFrame},
    recursive::collapse::Collapsable,
};

/// The ability to collapse a value into some output type, frame by frame
pub trait CollapsableAsync: Collapsable<FrameToken = Self::AsyncFrameToken>
where
    Self: Sized,
{
    type AsyncFrameToken: AsyncMappableFrame;

    /// defined on trait for convenience and to allow for optimized impls
    fn collapse_frames_async<Out, E>(
        self,
        collapse_frame: impl Fn(
                <Self::AsyncFrameToken as MappableFrame>::Frame<Out>,
            ) -> BoxFuture<'static, Result<Out, E>>
            + Send
            + Sync
            + 'static,
    ) -> BoxFuture<'static, Result<Out, E>>
    where
        Self: Send + Sync + 'static,
        Out: Send + Sync + 'static,
        <Self::AsyncFrameToken as MappableFrame>::Frame<Self>: Send + Sync + 'static,
        <Self::AsyncFrameToken as MappableFrame>::Frame<Out>: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        // expand_and_collapse_async::<Self, Out, E, Self::AsyncFrameToken>(
        //     self,
        //     Arc::new(|seed| std::future::ready(Ok(Self::into_frame(seed))).boxed()),
        //     Arc::new(collapse_frame),
        // )
        // .boxed()

        expand_and_collapse_async_new_2::<Self, Out, E, Self::AsyncFrameToken>(
            self,
            |seed| std::future::ready(Ok(Self::into_frame(seed))).boxed(),
            collapse_frame,
        ).boxed()
    }
}
