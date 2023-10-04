use crate::frame::{expand_and_collapse, MappableFrame};

/// The ability to recursively expand a seed to construct a value of this type, frame by frame.
/// For example:
/// For example:
///
/// ```rust
/// use recursion_schemes::frame::{MappableFrame, PartiallyApplied};
/// use recursion_schemes::Expandable;
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
///
/// impl Expandable for IntTree {
///     type FrameToken = IntTreeFrame<PartiallyApplied>;
///
///     fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self {
///         match val {
///             IntTreeFrame::Leaf { value } => IntTree::Leaf { value },
///             IntTreeFrame::Node { left, right } => IntTree::Node {
///                 left: Box::new(left),
///                 right: Box::new(right),
///             },
///         }
///     }
/// }
/// ```

pub trait Expandable
where
    Self: Sized,
{
    type FrameToken: MappableFrame;

    /// Given a frame holding instances of 'Self', generate an instance of 'Self'
    fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self;

    /// Given a value of type 'In', expand it to generate a value of type 'Self' frame by frame,
    /// using a function from 'In -> Frame<In>'
    ///
    /// defined on trait for convenience and to allow for optimized impls
    fn expand_frames<In>(
        input: In,
        expand_frame: impl FnMut(In) -> <Self::FrameToken as MappableFrame>::Frame<In>,
    ) -> Self {
        expand_and_collapse::<Self::FrameToken, In, Self>(input, expand_frame, Self::from_frame)
    }
}
