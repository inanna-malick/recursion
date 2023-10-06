use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use recursion::{experimental::compact::Compact, ExpandableExt, CollapsibleExt};
use recursion_tests::expr::{
    eval::{eval_layer, naive_eval},
    naive::Expr,
    ExprFrame,
};

fn bench_eval(criterion: &mut Criterion) {
    let mut test_cases = Vec::new();

    // build some Big Expressions that are Pointless and Shitty
    for depth in 17..18 {
        let big_expr = Expr::expand_frames(depth, |x| {
            if x > 0 {
                ExprFrame::Add(x - 1, x - 1)
            } else {
                ExprFrame::LiteralInt(0)
            }
        });

        let boxed_big_compact = Compact::new(big_expr.clone());

        // println!("heap size for depth {}: dfs {}", big_expr_dfs.len);

        test_cases.push((depth, Box::new(big_expr), boxed_big_compact));
    }

    let mut group = criterion.benchmark_group("evaluate expression tree");

    // let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    // group.plot_config(plot_config);

    for (depth, boxed_big_expr, boxed_big_compact) in test_cases.into_iter() {
        group.bench_with_input(
            BenchmarkId::new("traditional boxed method", depth),
            &boxed_big_expr,
            |b, expr| b.iter(|| naive_eval(expr)),
        );

        group.bench_with_input(
            BenchmarkId::new("fold stack_machine lazy with new GAT-based model", depth),
            &boxed_big_expr,
            |b, expr| b.iter(|| expr.as_ref().collapse_frames(eval_layer)),
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
