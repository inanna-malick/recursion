use std::fmt::DebugSet;

use crate::examples::expr::Expr;
use crate::examples::expr::{BlocAllocExpr, DFSStackExpr};

#[cfg(test)]
use crate::examples::expr::naive::arb_expr;
use crate::examples::expr::naive::{generate_layer, ExprAST};
use crate::recursive_traits::{CoRecursive, Recursive};
#[cfg(test)]
use proptest::prelude::*;

#[inline(always)]
pub fn eval_layer(node: Expr<i64>) -> i64 {
    match node {
        Expr::Add(a, b) => a + b,
        Expr::Sub(a, b) => a - b,
        Expr::Mul(a, b) => a * b,
        Expr::LiteralInt(x) => x,
    }
}

pub fn naive_eval(expr: &ExprAST) -> i64 {
    match expr {
        ExprAST::Add(a, b) => naive_eval(a) + naive_eval(b),
        ExprAST::Sub(a, b) => naive_eval(a) - naive_eval(b),
        ExprAST::Mul(a, b) => naive_eval(a) * naive_eval(b),
        ExprAST::LiteralInt(x) => *x,
    }
}

// generate a bunch of expression trees and evaluate them
#[cfg(test)]
proptest! {
    #[test]
    fn expr_eval(expr in arb_expr()) {
        // NOTE: this helped me find one serious bug in new cata impl, where it was doing vec pop instead of vec head_pop so switched to VecDequeue. Found minimal example, Add (0, Sub(0, 1)).
        let expr = Box::new(expr);
        let simple = naive_eval(&expr);
        let dfs_stack_eval = DFSStackExpr::unfold(expr.clone(), generate_layer).fold(eval_layer);
        let bloc_alloc_eval = BlocAllocExpr::unfold(expr, generate_layer).fold(eval_layer);


        assert_eq!(simple, dfs_stack_eval);
        assert_eq!(simple, bloc_alloc_eval);
    }
}
