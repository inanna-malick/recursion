use crate::frame::{expand_and_collapse, MappableFrame};

/// The ability to collapse a value into some output type, frame by frame
pub trait Collapsable
where
    Self: Sized,
{
    type FrameToken: MappableFrame;

    fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self>;

    /// defined on trait for convenience and to allow for optimized impls
    fn collapse_frames<Out>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Out,
    ) -> Out {
        expand_and_collapse::<Self::FrameToken, Self, Out>(self, Self::into_frame, collapse_frame)
    }
}
