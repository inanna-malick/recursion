use crate::{examples::expr::*, map_layer::Project, recursive::gat::Recursive};
#[cfg(test)]
use proptest::prelude::*;

/// simple naive representation of a recursive expression AST.
#[derive(Debug, Clone)]
pub enum ExprAST {
    Add(Box<ExprAST>, Box<ExprAST>),
    Sub(Box<ExprAST>, Box<ExprAST>),
    Mul(Box<ExprAST>, Box<ExprAST>),
    LiteralInt(i64),
}

impl Recursive for &ExprAST {
    type FunctorToken = Expr<PartiallyApplied>;

    fn into_layer(self) -> <Self::FunctorToken as crate::recursive::gat::Functor>::Layer<Self> {
        match self {
            ExprAST::Add(a, b) => Expr::Add(a, b),
            ExprAST::Sub(a, b) => Expr::Sub(a, b),
            ExprAST::Mul(a, b) => Expr::Mul(a, b),
            ExprAST::LiteralInt(x) => Expr::LiteralInt(*x),
        }
    }
}

pub fn generate_layer(x: &ExprAST) -> Expr<&ExprAST> {
    match x {
        ExprAST::Add(a, b) => Expr::Add(a, b),
        ExprAST::Sub(a, b) => Expr::Sub(a, b),
        ExprAST::Mul(a, b) => Expr::Mul(a, b),
        ExprAST::LiteralInt(x) => Expr::LiteralInt(*x),
    }
}

impl Project for &ExprAST {
    type To = Expr<Self>;

    fn project(self) -> Self::To {
        generate_layer(self)
    }
}

#[cfg(test)]
pub fn arb_expr() -> impl Strategy<Value = ExprAST> {
    let leaf = prop_oneof![any::<i8>().prop_map(|x| ExprAST::LiteralInt(x as i64)),];
    leaf.prop_recursive(
        8,   // 8 levels deep
        256, // Shoot for maximum size of 256 nodes
        10,  // We put up to 10 items per collection
        |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| ExprAST::Add(Box::new(a), Box::new(b))),
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| ExprAST::Sub(Box::new(a), Box::new(b))),
                (inner.clone(), inner).prop_map(|(a, b)| ExprAST::Mul(Box::new(a), Box::new(b))),
            ]
        },
    )
}
