pub mod eval;
pub mod monomorphic;
pub mod naive;

use futures::FutureExt;
use recursion::{
    map_layer::MapLayer,
    recursive_tree::{arena_eval::ArenaIndex, stack_machine_eval::StackMarker, RecursiveTree},
};
use recursion_schemes::{
    experimental::frame::{AsyncMappableFrame, MappableFrameRef},
    frame::MappableFrame,
};

/// Simple expression language with some operations on integers
#[derive(Debug, Clone, Copy)]
pub enum Expr<A> {
    Add(A, A),
    Sub(A, A),
    Mul(A, A),
    LiteralInt(i64),
}

pub enum ExprFrameToken {}

impl MappableFrame for ExprFrameToken {
    type Frame<X> = Expr<X>;

    #[inline(always)]
    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
        match input {
            Expr::Add(a, b) => Expr::Add(f(a), f(b)),
            Expr::Sub(a, b) => Expr::Sub(f(a), f(b)),
            Expr::Mul(a, b) => Expr::Mul(f(a), f(b)),
            Expr::LiteralInt(x) => Expr::LiteralInt(x),
        }
    }
}

impl<A> Expr<A> {
    async fn map_async<'a, B, E>(
        input: Expr<A>,
        mut f: impl FnMut(A) -> futures::future::BoxFuture<'a, Result<B, E>>,
    ) -> Result<Expr<B>, E> {
        match input {
            Expr::Add(a, b) => {
                let (a, b) = tokio::try_join!(f(a), f(b))?;
                Ok(Expr::Add(a, b))
            }
            Expr::Sub(a, b) => {
                let (a, b) = tokio::try_join!(f(a), f(b))?;
                Ok(Expr::Sub(a, b))
            }
            Expr::Mul(a, b) => {
                let (a, b) = tokio::try_join!(f(a), f(b))?;
                Ok(Expr::Mul(a, b))
            }
            Expr::LiteralInt(x) => Ok(Expr::LiteralInt(x)),
        }
    }
}

impl AsyncMappableFrame for ExprFrameToken {
    fn map_frame_async<'a, A, B, E>(
        input: Self::Frame<A>,
        f: impl Fn(A) -> futures::future::BoxFuture<'a, Result<B, E>> + Send + Sync + 'a,
    ) -> futures::future::BoxFuture<'a, Result<Self::Frame<B>, E>>
    where
        E: Send + 'a,
        A: Send + 'a,
        B: Send + 'a,
    {
        async { Expr::map_async(input, f).await }.boxed()
    }
}

// used for testing experimental 'Compact' repr
impl MappableFrameRef for ExprFrameToken {
    type RefFrameToken<'a> = ExprFrameToken; // token doesn't actually own any data

    // NOTE: the frame fn here is only actually used with 'A' == () and 'B' == Out
    #[inline(always)]
    fn as_ref<'a, X>(
        input: &'a Self::Frame<X>,
    ) -> <Self::RefFrameToken<'a> as MappableFrame>::Frame<&'a X> {
        match input {
            Expr::Add(a, b) => Expr::Add(a, b),
            Expr::Sub(a, b) => Expr::Sub(a, b),
            Expr::Mul(a, b) => Expr::Mul(a, b),
            Expr::LiteralInt(x) => Expr::LiteralInt(*x),
        }
    }
}

impl<A, B> MapLayer<B> for Expr<A> {
    type To = Expr<B>;
    type Unwrapped = A;

    #[inline(always)]
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            Expr::Add(a, b) => Expr::Add(f(a), f(b)),
            Expr::Sub(a, b) => Expr::Sub(f(a), f(b)),
            Expr::Mul(a, b) => Expr::Mul(f(a), f(b)),
            Expr::LiteralInt(x) => Expr::LiteralInt(x),
        }
    }
}

// this is, like, basically fine? - just usize and ()
impl<'a, A: Copy, B: 'a> MapLayer<B> for &'a Expr<A> {
    type To = Expr<B>;
    type Unwrapped = A;

    #[inline(always)]
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            Expr::Add(a, b) => Expr::Add(f(*a), f(*b)),
            Expr::Sub(a, b) => Expr::Sub(f(*a), f(*b)),
            Expr::Mul(a, b) => Expr::Mul(f(*a), f(*b)),
            Expr::LiteralInt(x) => Expr::LiteralInt(*x),
        }
    }
}

pub type DFSStackExpr = RecursiveTree<Expr<StackMarker>, StackMarker>;
pub type BlocAllocExpr = RecursiveTree<Expr<ArenaIndex>, ArenaIndex>;
