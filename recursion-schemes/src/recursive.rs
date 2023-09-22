use crate::frame::{expand_compact, MappableFrame, MappableFrameRef};

use self::collapse::Collapsable;

pub mod collapse;
pub mod expand;

// impl<'a, X: MappableFrameRef> MappableFrame for &'a X {
//     // problem! needs 'a bound so can't write this
//     type Frame<Next> = <X as MappableFrameRef>::Frame<'a, Next> where X: 'a;

//     fn map_frame<A, B>(input: Self::Frame<A>, f: impl FnMut(A) -> B) -> Self::Frame<B> {
//         <X as MappableFrameRef>::map_frame(&input, f)
//     }
// }

// TODO: move all of this under the experimental flag
pub struct Compact<F: MappableFrame>(pub Vec<F::Frame<()>>);

#[repr(transparent)]
pub struct CompactRef<F: MappableFrame>(pub [F::Frame<()>]);

impl<F: MappableFrame> Compact<F> {
    // the idea here is to have 'compact' as a transparent wrapper around collapsable structures,
    // such that they can be pre-compacted and we don't need to run the expand step each time

    // ALSO, this makes it so we can just remove the expandable/collapsable defn's and can
    // just have a method 'collapse_frames' on Compact
    pub fn compact<E: Collapsable<FrameToken = F>>(e: E) -> Self {
        expand_compact(e, E::into_frame)
    }

    pub fn collapse_frames<Out>(
        self,
        collapse_frame: impl FnMut(<F as MappableFrame>::Frame<Out>) -> Out,
    ) -> Out {
        crate::frame::collapse_compact::<F, Out>(self, collapse_frame)
    }
}

impl<F: MappableFrame + MappableFrameRef> Compact<F> {
    pub fn collapse_frames_ref<'a, 'c: 'a, Out>(
        &'c self,
        collapse_frame: impl FnMut(<F::RefFrameToken<'a> as MappableFrame>::Frame<Out>) -> Out,
    ) -> Out {
        crate::frame::collapse_compact_ref::<'a, 'c, F, Out>(self, collapse_frame)
    }
}

impl<F: MappableFrame> collapse::Collapsable for Compact<F> {
    type FrameToken = F;

    // TODO: unify below functions? seems like a strong yes
    fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
        unimplemented!("not used")
    }

    fn collapse_frames<Out>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Out,
    ) -> Out {
        crate::frame::collapse_compact::<Self::FrameToken, Out>(self, collapse_frame)
    }
}

impl<F: MappableFrame> expand::Expandable for Compact<F> {
    type FrameToken = F;

    fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self {
        unimplemented!("not used")
    }

    fn expand_frames<In>(
        input: In,
        expand_frame: impl FnMut(In) -> <Self::FrameToken as MappableFrame>::Frame<In>,
    ) -> Self {
        crate::frame::expand_compact::<Self::FrameToken, In>(input, expand_frame)
    }
}

// impl<F: MappableFrameRef> collapse::CollapsableRef for Compact<F> {
//     fn into_frame<'a>(&'a self) -> <Self::FrameToken as MappableFrameRef>::RefFrame<'a, Self> {
//         todo!()
//     }

//     // fn collapse_frames<Out>(
//     //     self,
//     //     collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Out,
//     // ) -> Out {
//     //     crate::frame::collapse_compact_ref::<Self::FrameToken, Out>(self, collapse_frame)
//     // }
// }
