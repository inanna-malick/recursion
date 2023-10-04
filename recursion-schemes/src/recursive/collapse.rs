use crate::frame::{expand_and_collapse, MappableFrame};

/// The ability to recursively collapse some type into some output type, frame by frame.
/// For example:
///
/// ```rust
/// use recursion_schemes::frame::{MappableFrame, PartiallyApplied};
/// use recursion_schemes::Collapsable;
///
/// enum IntTreeFrame<A> {
///     Leaf { value: usize },
///     Node { left: A, right: A },
/// }
/// impl MappableFrame for IntTreeFrame<PartiallyApplied> {
///     type Frame<X> = IntTreeFrame<X>;
///     
///     fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
///       unimplemented!("elided")
///     }
/// }
///
/// enum IntTree {
///     Leaf { value: usize },
///     Node { left: Box<Self>, right: Box<Self> },
/// }
////
/// impl<'a> Collapsable for &'a IntTree {
///     type FrameToken = IntTreeFrame<PartiallyApplied>;
///
///     fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
///         match self {
///             IntTree::Leaf { value } => IntTreeFrame::Leaf { value: *value },
///             IntTree::Node { left, right } => IntTreeFrame::Node {
///                 left: left.as_ref(),
///                 right: right.as_ref(),
///             },
///         }
///     }
/// }
/// ```

pub trait Collapsable
where
    Self: Sized,
{
    type FrameToken: MappableFrame;

    /// Given an instance of this type, generate a frame holding the data owned by it,
    /// with any recursive instances of 'Self' owned by this node as the frame elements
    fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self>;

    /// Given an instance of this type, collapse it into a single value of type 'Out' by
    /// traversing the recursive structure of 'self', generating frames, and collapsing
    /// those frames using some function from 'Frame<Out> -> Out'
    ///
    /// This function is defined on the 'Collapse' trait for convenience and to allow for optimized impls
    fn collapse_frames<Out>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Out,
    ) -> Out {
        expand_and_collapse::<Self::FrameToken, Self, Out>(self, Self::into_frame, collapse_frame)
    }
}
