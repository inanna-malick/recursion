use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use schemes::examples::expr::{
    eval::{eval, naive_eval},
    naive::{from_ast, ExprAST},
    RecursiveExpr,
};

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
    // TODO: deliberately fuck up pointer locality
    let a = build_simple_expr();
    let b = build_simple_expr();
    let c = build_simple_expr();
    let d = build_simple_expr();
    let expr_ast = build_simple_expr2(a, b, c, d);

    let recursive_expr_ast = from_ast(Box::new(expr_ast.clone()));

    let h = HashMap::new();

    criterion.bench_function("eval boxed", |b| {
        b.iter(|| naive_eval(&h, black_box(&expr_ast)))
    });
    criterion.bench_function("eval fold", |b| {
        b.iter(|| eval(&h, black_box(&recursive_expr_ast)))
    });

}

criterion_group!(benches, bench_fib);
criterion_main!(benches);
