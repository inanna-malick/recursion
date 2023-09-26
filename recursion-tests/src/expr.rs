pub mod eval;
pub mod monomorphic;
pub mod naive;

use recursion::{
    map_layer::MapLayer,
    recursive_tree::{arena_eval::ArenaIndex, stack_machine_eval::StackMarker, RecursiveTree},
};
use recursion_schemes::{experimental::frame::MappableFrameRef, frame::MappableFrame};

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

// impl JoinFuture for Expr<PartiallyApplied> {
//     type FunctorToken = Expr<PartiallyApplied>;

//     fn join_layer<A: Send + 'static>(
//         input: <<Self as JoinFuture>::FunctorToken as Functor>::Layer<BoxFuture<'static, A>>,
//     ) -> BoxFuture<'static, <<Self as JoinFuture>::FunctorToken as Functor>::Layer<A>> {
//         async {
//             use futures::future::join;
//             match input {
//                 Expr::Add(a, b) => {
//                     let (a, b) = join(a, b).await;
//                     Expr::Add(a, b)
//                 }
//                 Expr::Sub(a, b) => {
//                     let (a, b) = join(a, b).await;
//                     Expr::Sub(a, b)
//                 }
//                 Expr::Mul(a, b) => {
//                     let (a, b) = join(a, b).await;
//                     Expr::Mul(a, b)
//                 }
//                 Expr::LiteralInt(x) => Expr::LiteralInt(x),
//             }
//         }
//         .boxed()
//     }
// }

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
