use crate::expr::*;
use proptest::prelude::*;
use recursion::{
    experimental::recursive::collapse::CollapsibleAsync, Collapsible, Expandable, PartiallyApplied,
};

/// simple naive representation of a recursive expression AST.
#[derive(Debug, Clone)]
pub enum Expr {
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    LiteralInt(i64),
}

impl<'a> Collapsible for &'a Expr {
    type FrameToken = ExprFrame<PartiallyApplied>;

    #[inline(always)]
    fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
        match self {
            Expr::Add(a, b) => ExprFrame::Add(a, b),
            Expr::Sub(a, b) => ExprFrame::Sub(a, b),
            Expr::Mul(a, b) => ExprFrame::Mul(a, b),
            Expr::LiteralInt(x) => ExprFrame::LiteralInt(*x),
        }
    }
}

impl Collapsible for Expr {
    type FrameToken = ExprFrame<PartiallyApplied>;

    #[inline(always)]
    fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
        match self {
            Expr::Add(a, b) => ExprFrame::Add(*a, *b),
            Expr::Sub(a, b) => ExprFrame::Sub(*a, *b),
            Expr::Mul(a, b) => ExprFrame::Mul(*a, *b),
            Expr::LiteralInt(x) => ExprFrame::LiteralInt(x),
        }
    }
}

impl Expandable for Expr {
    type FrameToken = ExprFrame<PartiallyApplied>;

    fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self {
        match val {
            ExprFrame::Add(a, b) => Expr::Add(Box::new(a), Box::new(b)),
            ExprFrame::Sub(a, b) => Expr::Sub(Box::new(a), Box::new(b)),
            ExprFrame::Mul(a, b) => Expr::Mul(Box::new(a), Box::new(b)),
            ExprFrame::LiteralInt(x) => Expr::LiteralInt(x),
        }
    }
}

impl CollapsibleAsync for Expr {
    type AsyncFrameToken = ExprFrame<PartiallyApplied>;
}

pub fn generate_layer(x: &Expr) -> ExprFrame<&Expr> {
    match x {
        Expr::Add(a, b) => ExprFrame::Add(a, b),
        Expr::Sub(a, b) => ExprFrame::Sub(a, b),
        Expr::Mul(a, b) => ExprFrame::Mul(a, b),
        Expr::LiteralInt(x) => ExprFrame::LiteralInt(*x),
    }
}

pub fn arb_expr() -> impl Strategy<Value = Expr> {
    let leaf = prop_oneof![any::<i8>().prop_map(|x| Expr::LiteralInt(x as i64)),];
    leaf.prop_recursive(
        8,   // 8 levels deep
        256, // Shoot for maximum size of 256 nodes
        10,  // We put up to 10 items per collection
        |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| Expr::Add(Box::new(a), Box::new(b))),
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| Expr::Sub(Box::new(a), Box::new(b))),
                (inner.clone(), inner).prop_map(|(a, b)| Expr::Mul(Box::new(a), Box::new(b))),
            ]
        },
    )
}
