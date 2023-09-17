use crate::frame::{MappableFrame, MappableFrameExt};

use super::HasRecursiveFrame;

/// The ability to collapse a value into some output type, frame by frame
pub trait CollapseRecursive: HasRecursiveFrame {
    fn collapse_recursive<Out>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Out,
    ) -> Out;
}

pub trait IntoRecursiveFrame
where
    Self: Sized,
{
    type FrameToken: MappableFrame;

    fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self>;
}

impl<X> CollapseRecursive for X
where
    X: HasRecursiveFrame,
    X: IntoRecursiveFrame<FrameToken = <X as HasRecursiveFrame>::FrameToken>,
{
    fn collapse_recursive<Out>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Out,
    ) -> Out {
        Self::FrameToken::expand_and_collapse(self, X::into_frame, collapse_frame)
    }
}
