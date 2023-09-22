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

// TODO: figure out what this needs to look like
// The ability to collapse a value into some output type, frame by frame
// pub trait CollapsableRef: HasRecursiveFrameRef
// where
//     Self: Sized,
// {
//     fn into_frame<'a>(&'a self) -> <<Self::FrameToken as MappableFrameRef>::RefFrameToken<'a> as MappableFrame>::Frame<&'a Self>;

//     /// defined on trait for convenience and to allow for optimized impls
//     fn collapse_frames_ref<'a, Out>(
//         &'a self,
//         collapse_frame: impl FnMut(
//             <<Self::FrameToken as MappableFrameRef>::RefFrameToken<'a> as MappableFrame>::Frame<Out>,
//         ) -> Out,
//     ) -> Out {
//         expand_and_collapse_ref::<Self::FrameToken, Self, Out>(self, Self::into_frame, collapse_frame)
//     }
// }
