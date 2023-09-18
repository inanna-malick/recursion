use crate::frame::{expand_and_collapse, MappableFrame};

use super::HasRecursiveFrame;

pub trait Expandable: HasRecursiveFrame
where
    Self: Sized,
{
    /// can' think of what to write here
    fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self;

    /// defined on trait for convenience and to allow for optimized impls
    fn expand_frames<In>(
        input: In,
        expand_frame: impl FnMut(In) -> <Self::FrameToken as MappableFrame>::Frame<In>,
    ) -> Self {
        expand_and_collapse::<Self::FrameToken, In, Self>(input, expand_frame, Self::from_frame)
    }
}