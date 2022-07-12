use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use schemes::{examples::expr::{
    eval::{eval, naive_eval},
    naive::{from_ast, ExprAST},
    RecursiveExpr, Expr,
}, recursive::{CoRecursive, Recursive}};

fn build_simple_expr() -> Box<ExprAST> {
    Box::new(ExprAST::Mul(
        Box::new(ExprAST::Add(
            Box::new(ExprAST::LiteralInt(1)),
            Box::new(ExprAST::LiteralInt(2)),
        )),
        Box::new(ExprAST::Sub(
            Box::new(ExprAST::LiteralInt(3)),
            Box::new(ExprAST::LiteralInt(4)),
        )),
    ))
}

fn build_simple_expr2(
    a: Box<ExprAST>,
    b: Box<ExprAST>,
    c: Box<ExprAST>,
    d: Box<ExprAST>,
) -> ExprAST {
    ExprAST::Mul(Box::new(ExprAST::Add(a, b)), Box::new(ExprAST::Sub(c, d)))
}

fn bench_fib(criterion: &mut Criterion) {

    // build a Big Expression that is Pointless and Shitty
    let big_expr = RecursiveExpr::unfold(20 as usize, |x| {
        if x > 0 {
            Expr::Add(x - 1, x - 1)
        } else {
            Expr::LiteralInt(0)
        }
    });

    let boxed_big_expr = big_expr.as_ref().fold(|n| match n {
        Expr::Add(a, b) => Box::new(ExprAST::Add(a, b)),
        Expr::Sub(a, b) => Box::new(ExprAST::Sub(a, b)),
        Expr::Mul(a, b) => Box::new(ExprAST::Mul(a, b)),
        Expr::LiteralInt(x) => Box::new(ExprAST::LiteralInt(x)),
        Expr::DatabaseRef(x) => Box::new(ExprAST::DatabaseRef(x)),
    });


    let h = HashMap::new();

    criterion.bench_function("eval boxed", |b| {
        b.iter(|| naive_eval(&h, black_box(&boxed_big_expr)))
    });
    criterion.bench_function("eval fold", |b| {
        b.iter(|| eval(&h, black_box(&big_expr)))
    });

}

criterion_group!(benches, bench_fib);
criterion_main!(benches);
