use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use recursion::recursive::{Collapse, Expand};
use recursion_schemes::{experimental::compact::Compact, recursive::collapse::Collapsable};
use recursion_tests::expr::{
    eval::{eval_layer, eval_lazy, eval_lazy_with_fused_compile, naive_eval},
    naive::ExprAST,
    BlocAllocExpr, DFSStackExpr, Expr, ExprFrameToken,
};

fn bench_eval(criterion: &mut Criterion) {
    let mut test_cases = Vec::new();

    // build some Big Expressions that are Pointless and Shitty
    for depth in 17..18 {
        let big_expr_bloc_alloc = BlocAllocExpr::expand_layers(depth, |x| {
            if x > 0 {
                Expr::Add(x - 1, x - 1)
            } else {
                Expr::LiteralInt(0)
            }
        });

        let big_expr_dfs = DFSStackExpr::expand_layers(depth, |x| {
            if x > 0 {
                Expr::Add(x - 1, x - 1)
            } else {
                Expr::LiteralInt(0)
            }
        });

        let boxed_big_expr = big_expr_dfs.as_ref().collapse_layers(|n| match n {
            Expr::Add(a, b) => Box::new(ExprAST::Add(a, b)),
            Expr::LiteralInt(x) => Box::new(ExprAST::LiteralInt(x)),
            _ => unreachable!(),
        });

        let boxed_big_compact = Compact::new(boxed_big_expr.clone().as_ref());

        // println!("heap size for depth {}: dfs {}", big_expr_dfs.len);

        test_cases.push((
            depth,
            big_expr_bloc_alloc,
            big_expr_dfs,
            boxed_big_expr,
            boxed_big_compact,
        ));
    }

    let mut group = criterion.benchmark_group("evaluate expression tree");

    // let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    // group.plot_config(plot_config);

    for (depth, big_expr_bloc_alloc, big_expr_dfs, boxed_big_expr, boxed_big_compact) in
        test_cases.into_iter()
    {
        group.bench_with_input(
            BenchmarkId::new("traditional boxed method", depth),
            &boxed_big_expr,
            |b, expr| b.iter(|| naive_eval(expr)),
        );
        group.bench_with_input(
            BenchmarkId::new("my new fold method", depth),
            &big_expr_bloc_alloc,
            |b, expr| b.iter(|| expr.as_ref().collapse_layers(eval_layer)),
        );
        group.bench_with_input(
            BenchmarkId::new("fold dfs stack", depth),
            &big_expr_dfs,
            |b, expr| b.iter(|| expr.as_ref().collapse_layers(eval_layer)),
        );
        group.bench_with_input(
            BenchmarkId::new("fold stack_machine lazy", depth),
            &boxed_big_expr,
            |b, expr| b.iter(|| eval_lazy(expr)),
        );
        group.bench_with_input(
            BenchmarkId::new("fold stack_machine lazy with fused compile", depth),
            &boxed_big_expr,
            |b, expr| b.iter(|| eval_lazy_with_fused_compile(expr)),
        );

        group.bench_with_input(
            BenchmarkId::new("fold stack_machine lazy with new GAT-based model", depth),
            &boxed_big_expr,
            |b, expr| b.iter(|| expr.collapse_frames(eval_layer)),
        );

        group.bench_with_input(
            BenchmarkId::new("fold stack_machine lazy with new GAT-based compact", depth),
            &boxed_big_compact,
            |b, expr| b.iter(|| expr.collapse_frames_ref(eval_layer)),
        );
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .with_profiler(
            PProfProfiler::new(100, Output::Flamegraph(None))
        );
    targets = bench_eval
}
criterion_main!(benches);
