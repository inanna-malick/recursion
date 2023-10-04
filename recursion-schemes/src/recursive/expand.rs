use crate::frame::{expand_and_collapse, MappableFrame};

/// The ability to recursively expand a seed to construct a value of this type, frame by frame.
/// For example, a tree of integers:
///
/// ```rust
/// # use recursion_schemes::frame::{MappableFrame, PartiallyApplied};
/// #[derive(Debug, PartialEq, Eq)]
/// enum IntTree {
///     Leaf { value: usize },
///     Node { left: Box<Self>, right: Box<Self> },
/// }
///
/// # impl IntTree {
/// #   fn node(left: Self, right: Self) -> Self { Self::Node{left: Box::new(left), right: Box::new(right)}}
/// #   fn leaf(value: usize) -> Self { Self::Leaf{value}}
/// # }
/// ```
///
/// We'll use `IntTreeFrame<A>` for working with `IntTree`s
/// ```rust
/// # use recursion_schemes::frame::{MappableFrame, PartiallyApplied};
/// enum IntTreeFrame<A> {
///     Leaf { value: usize },
///     Node { left: A, right: A },
/// }
/// impl MappableFrame for IntTreeFrame<PartiallyApplied> { /*...*/
/// #    type Frame<X> = IntTreeFrame<X>;
/// #
/// #    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
/// #         match input {
/// #             IntTreeFrame::Leaf { value } => IntTreeFrame::Leaf { value },
/// #             IntTreeFrame::Node { left, right } => IntTreeFrame::Node {
/// #                 left: f(left),
/// #                 right: f(right),
/// #             },
/// #         }
/// #     }
/// }
/// ```
/// Then we can define an `Expandable` instance for `IntTree`
///
/// ```rust
/// # use recursion_schemes::frame::{MappableFrame, PartiallyApplied};
/// # use recursion_schemes::Expandable;
/// # #[derive(Debug, PartialEq, Eq)]
/// # enum IntTree {
/// #     Leaf { value: usize },
/// #     Node { left: Box<Self>, right: Box<Self> },
/// # }
/// # enum IntTreeFrame<A> {
/// #     Leaf { value: usize },
/// #     Node { left: A, right: A },
/// # }
/// # impl MappableFrame for IntTreeFrame<PartiallyApplied> {
/// #    type Frame<X> = IntTreeFrame<X>;
/// #
/// #    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
/// #         match input {
/// #             IntTreeFrame::Leaf { value } => IntTreeFrame::Leaf { value },
/// #             IntTreeFrame::Node { left, right } => IntTreeFrame::Node {
/// #                 left: f(left),
/// #                 right: f(right),
/// #             },
/// #         }
/// #     }
/// # }
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
///
/// Finally, we can use our `Expandable` instance to generate a tree
///
/// ```rust
/// # use recursion_schemes::frame::{MappableFrame, PartiallyApplied};
/// # use recursion_schemes::Expandable;
/// # #[derive(Debug, PartialEq, Eq)]
/// # enum IntTree {
/// #     Leaf { value: usize },
/// #     Node { left: Box<Self>, right: Box<Self> },
/// # }
/// # impl IntTree {
/// #   fn node(left: Self, right: Self) -> Self { Self::Node{left: Box::new(left), right: Box::new(right)}}
/// #   fn leaf(value: usize) -> Self { Self::Leaf{value}}
/// # }
/// # enum IntTreeFrame<A> {
/// #     Leaf { value: usize },
/// #     Node { left: A, right: A },
/// # }
/// # impl MappableFrame for IntTreeFrame<PartiallyApplied> {
/// #    type Frame<X> = IntTreeFrame<X>;
/// #
/// #    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
/// #         match input {
/// #             IntTreeFrame::Leaf { value } => IntTreeFrame::Leaf { value },
/// #             IntTreeFrame::Node { left, right } => IntTreeFrame::Node {
/// #                 left: f(left),
/// #                 right: f(right),
/// #             },
/// #         }
/// #     }
/// # }
/// # impl Expandable for IntTree {
/// #     type FrameToken = IntTreeFrame<PartiallyApplied>;
/// #
/// #     fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self {
/// #         match val {
/// #             IntTreeFrame::Leaf { value } => IntTree::Leaf { value },
/// #             IntTreeFrame::Node { left, right } => IntTree::Node {
/// #                 left: Box::new(left),
/// #                 right: Box::new(right),
/// #             },
/// #         }
/// #     }
/// # }
/// let depth = 2;
///
/// let expanded_tree = IntTree::expand_frames(depth, |n| {
///     if n <= 0 {
///         IntTreeFrame::Leaf { value: n }
///     } else {
///         IntTreeFrame::Node {
///             left: n - 1,
///             right: n - 1,
///         }
///     }
/// });
///
/// let expected = IntTree::node(
///     IntTree::node(IntTree::leaf(0), IntTree::leaf(0)),
///     IntTree::node(IntTree::leaf(0), IntTree::leaf(0)),
/// );
///
/// assert_eq!(expected, expanded_tree)
/// ```

pub trait Expandable
where
    Self: Sized,
{
    type FrameToken: MappableFrame;

    /// Given a frame holding instances of `Self`, generate an instance of `Self`
    fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self;

    /// Given a value of type `In`, expand it to generate a value of type `Self` frame by frame,
    /// using a function from `In -> Frame<In>`
    ///
    /// defined on trait for convenience and to allow for optimized impls
    fn expand_frames<In>(
        input: In,
        expand_frame: impl FnMut(In) -> <Self::FrameToken as MappableFrame>::Frame<In>,
    ) -> Self {
        expand_and_collapse::<Self::FrameToken, In, Self>(input, expand_frame, Self::from_frame)
    }
}
