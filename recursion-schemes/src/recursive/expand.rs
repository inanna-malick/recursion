use crate::frame::{MappableFrame, MappableFrameExt};

use super::HasRecursiveFrame;

pub trait FromRecursiveFrame
where
    Self: Sized,
{
    type FrameToken: MappableFrame;

    fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self;
}

pub trait Expand: HasRecursiveFrame {
    fn expand_frames<In>(
        input: In,
        expand_frame: impl FnMut(In) -> <Self::FrameToken as MappableFrame>::Frame<In>,
    ) -> Self;
}

impl<X> Expand for X
where
    X: HasRecursiveFrame,
    X: FromRecursiveFrame<FrameToken = <X as HasRecursiveFrame>::FrameToken>,
{
    fn expand_frames<In>(
        input: In,
        expand_frame: impl FnMut(In) -> <Self::FrameToken as MappableFrame>::Frame<In>,
    ) -> Self {
        Self::FrameToken::expand_and_collapse(input, expand_frame, X::from_frame)
    }
}
