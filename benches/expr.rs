use std::collections::HashMap;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, PlotConfiguration, AxisScale};
use schemes::{
    examples::expr::{
        eval::{eval, naive_eval},
        naive::ExprAST,
        Expr, RecursiveExpr,
    },
    recursive::{CoRecursive, Recursive},
};

fn bench_eval(criterion: &mut Criterion) {
    let mut test_cases = Vec::new();

    // build some Big Expressions that are Pointless and Shitty
    for depth in 0..18 {
        let big_expr = RecursiveExpr::unfold(depth, |x| {
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

        test_cases.push((depth, big_expr, boxed_big_expr));
    }

    let h = HashMap::new();

    let mut group = criterion.benchmark_group("eval");

    let plot_config = PlotConfiguration::default()
        .summary_scale(AxisScale::Logarithmic);
    group.plot_config(plot_config);

    for (depth, big_expr, boxed_big_expr) in test_cases.into_iter() {
        let elem_count = 2_usize.pow(depth);
        group.bench_with_input(
            BenchmarkId::new("boxed", elem_count),
            &boxed_big_expr,
            |b, expr|  b.iter(|| naive_eval(&h, &expr)),
        );
        group.bench_with_input(
            BenchmarkId::new("fold", elem_count),
            &big_expr,
            |b, expr| b.iter(|| eval(&h, expr)),
        );
    }
    group.finish();
}

criterion_group!(benches, bench_eval);
criterion_main!(benches);
