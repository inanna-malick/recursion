use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use schemes::{examples::expr::{
    eval::{eval, naive_eval},
    naive::{ ExprAST},
    RecursiveExpr, Expr,
}, recursive::{CoRecursive, Recursive}};


fn bench_fib(criterion: &mut Criterion) {

    // build a Big Expression that is Pointless and Shitty
    let big_expr = RecursiveExpr::unfold(13 as usize, |x| {
        if x > 0 {
            Expr::Add(x - 1, x - 1)
        } else {
            Expr::LiteralInt(0)
        }
    });

    let boxed_big_expr = big_expr.as_ref().fold(|n| match n {
        Expr::Add(a, b) => Box::new(ExprAST::Add(a, b)),
        Expr::LiteralInt(x) => Box::new(ExprAST::LiteralInt(x)),
        _ => unreachable!(),
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
