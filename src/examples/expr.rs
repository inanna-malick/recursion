pub mod eval;
#[cfg(test)]
pub mod monomorphic;
pub mod naive;

use crate::{
    map_layer::MapLayer,
    recursive_tree::{arena_eval::ArenaIndex, stack_machine_eval::StackMarker, RecursiveTree},
};

/// Simple expression language with some operations on integers
#[derive(Debug, Clone, Copy)]
pub enum Expr<A> {
    Add(A, A),
    Sub(A, A),
    Mul(A, A),
    LiteralInt(i64),
}

impl<A> MapLayer for Expr<A> {
    type Layer<B> = Expr<B>;
    type Unwrapped = A;

    #[inline(always)]
    fn map_layer<F: FnMut(Self::Unwrapped) -> B, B>(self, mut f: F) -> Self::Layer<B> {
        match self {
            Expr::Add(a, b) => Expr::Add(f(a), f(b)),
            Expr::Sub(a, b) => Expr::Sub(f(a), f(b)),
            Expr::Mul(a, b) => Expr::Mul(f(a), f(b)),
            Expr::LiteralInt(x) => Expr::LiteralInt(x),
        }
    }
}

// this is, like, basically fine? - just usize and ()
impl<'a, A: Copy> MapLayer for &'a Expr<A> {
    type Layer<B> = Expr<B>;
    type Unwrapped = A;

    #[inline(always)]
    fn map_layer<F: FnMut(Self::Unwrapped) -> B, B>(self, mut f: F) -> Self::Layer<B> {
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
