use crate::frame::{collapse_compact, MappableFrame, MappableFrameRef};

pub mod collapse;
pub mod expand;

/// A type with an associated frame type via which instances can be expanded or collapsed
pub trait HasRecursiveFrame {
    type FrameToken: MappableFrame;
}

// impl<'a, X: MappableFrameRef> MappableFrame for &'a X {
//     // problem! needs 'a bound so can't write this
//     type Frame<Next> = <X as MappableFrameRef>::Frame<'a, Next> where X: 'a;

//     fn map_frame<A, B>(input: Self::Frame<A>, f: impl FnMut(A) -> B) -> Self::Frame<B> {
//         <X as MappableFrameRef>::map_frame(&input, f)
//     }
// }

pub trait HasRecursiveFrameRef {
    type FrameToken: MappableFrameRef;
}

pub struct Compact<F: MappableFrame>(pub Vec<F::Frame<()>>);
pub struct CompactRef<'a, F: MappableFrameRef>(pub &'a [F::Frame<'a, ()>]);

impl<F: MappableFrame> HasRecursiveFrame for Compact<F> {
    type FrameToken = F;
}

impl<F: MappableFrame> collapse::Collapsable for Compact<F> {
    // TODO: unify below functions? seems like a strong yes
    fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
        unimplemented!("do not call")
    }

    fn collapse_frames<Out>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Out,
    ) -> Out {
        crate::frame::collapse_compact::<Self::FrameToken, Out>(self, collapse_frame)
    }
}

impl<F: MappableFrame> expand::Expandable for Compact<F> {
    fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self {
        todo!()
    }

    fn expand_frames<In>(
        input: In,
        expand_frame: impl FnMut(In) -> <Self::FrameToken as MappableFrame>::Frame<In>,
    ) -> Self {
        crate::frame::expand_compact::<Self::FrameToken, In>(input, expand_frame)
    }
}

// how tf does this even work lol
impl<'a, F: MappableFrameRef + 'a> HasRecursiveFrameRef for &'a Compact<F> {
    type FrameToken = F;
}

impl<'a, F: MappableFrameRef + 'a> collapse::CollapsableRef for &'a Compact<F> {
    fn into_frame<'a>(&'a self) -> <Self::FrameToken as MappableFrameRef>::Frame<'a, Self> {
        todo!()
    }



    // fn collapse_frames<Out>(
    //     self,
    //     collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Out,
    // ) -> Out {
    //     crate::frame::collapse_compact_ref::<Self::FrameToken, Out>(self, collapse_frame)
    // }
}
