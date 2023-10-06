pub mod eval;
pub mod monomorphic;
pub mod naive;

use futures::FutureExt;
use recursion::{
    experimental::frame::{AsyncMappableFrame, MappableFrameRef},
    MappableFrame, PartiallyApplied,
};

/// Simple expression language with some operations on integers
#[derive(Debug, Clone, Copy)]
pub enum ExprFrame<A> {
    Add(A, A),
    Sub(A, A),
    Mul(A, A),
    LiteralInt(i64),
}

impl MappableFrame for ExprFrame<PartiallyApplied> {
    type Frame<X> = ExprFrame<X>;

    #[inline(always)]
    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
        match input {
            ExprFrame::Add(a, b) => ExprFrame::Add(f(a), f(b)),
            ExprFrame::Sub(a, b) => ExprFrame::Sub(f(a), f(b)),
            ExprFrame::Mul(a, b) => ExprFrame::Mul(f(a), f(b)),
            ExprFrame::LiteralInt(x) => ExprFrame::LiteralInt(x),
        }
    }
}

impl<A> ExprFrame<A> {
    async fn map_async<'a, B, E>(
        input: ExprFrame<A>,
        mut f: impl FnMut(A) -> futures::future::BoxFuture<'a, Result<B, E>>,
    ) -> Result<ExprFrame<B>, E> {
        match input {
            ExprFrame::Add(a, b) => {
                let (a, b) = tokio::try_join!(f(a), f(b))?;
                Ok(ExprFrame::Add(a, b))
            }
            ExprFrame::Sub(a, b) => {
                let (a, b) = tokio::try_join!(f(a), f(b))?;
                Ok(ExprFrame::Sub(a, b))
            }
            ExprFrame::Mul(a, b) => {
                let (a, b) = tokio::try_join!(f(a), f(b))?;
                Ok(ExprFrame::Mul(a, b))
            }
            ExprFrame::LiteralInt(x) => Ok(ExprFrame::LiteralInt(x)),
        }
    }
}

impl AsyncMappableFrame for ExprFrame<PartiallyApplied> {
    fn map_frame_async<'a, A, B, E>(
        input: Self::Frame<A>,
        f: impl Fn(A) -> futures::future::BoxFuture<'a, Result<B, E>> + Send + Sync + 'a,
    ) -> futures::future::BoxFuture<'a, Result<Self::Frame<B>, E>>
    where
        E: Send + 'a,
        A: Send + 'a,
        B: Send + 'a,
    {
        async { ExprFrame::map_async(input, f).await }.boxed()
    }
}

// used for testing experimental 'Compact' repr
impl MappableFrameRef for ExprFrame<PartiallyApplied> {
    type RefFrameToken<'a> = ExprFrame<PartiallyApplied>; // token doesn't actually own any data

    // NOTE: the frame fn here is only actually used with 'A' == () and 'B' == Out
    #[inline(always)]
    fn as_ref<'a, X>(
        input: &'a Self::Frame<X>,
    ) -> <Self::RefFrameToken<'a> as MappableFrame>::Frame<&'a X> {
        match input {
            ExprFrame::Add(a, b) => ExprFrame::Add(a, b),
            ExprFrame::Sub(a, b) => ExprFrame::Sub(a, b),
            ExprFrame::Mul(a, b) => ExprFrame::Mul(a, b),
            ExprFrame::LiteralInt(x) => ExprFrame::LiteralInt(*x),
        }
    }
}
