use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use recursion_schemes::{frame::MappableFrame, recursive::collapse::Collapsable};

enum PartiallyApplied {}

pub enum ListFrame<Elem, Next> {
    Cons(Elem, Next),
    Nil,
}

impl<Elem> MappableFrame for ListFrame<Elem, PartiallyApplied> {
    type Frame<Next> = ListFrame<Elem, Next>;

    #[inline(always)]
    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
        match input {
            ListFrame::Cons(elem, next) => ListFrame::Cons(elem, f(next)),
            ListFrame::Nil => ListFrame::Nil,
        }
    }
}

struct CollapsableSlice<'a, Elem>(&'a [Elem]);

impl<'a, Elem: 'a> Collapsable for CollapsableSlice<'a, Elem> {
    type FrameToken = ListFrame<&'a Elem, PartiallyApplied>;

    #[inline(always)]
    fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
        match self.0.split_first() {
            Some((first, rest)) => ListFrame::Cons(first, CollapsableSlice(rest)),
            None => ListFrame::Nil,
        }
    }
}

fn bench_eval(criterion: &mut Criterion) {
    let mut bigvec = Vec::with_capacity(1024 * 1024);
    bigvec.resize(1024 * 1024, 1);
    let test_cases = vec![bigvec];

    let mut group = criterion.benchmark_group("sum_via_fold");

    for input in test_cases.into_iter() {
        group.bench_with_input(
            BenchmarkId::new("fold iter", input.len()),
            &input,
            |b, input| b.iter(|| input.iter().fold(0, |x, acc| x + acc)),
        );

        group.bench_with_input(
            BenchmarkId::new("fold_frames", input.len()),
            &input,
            |b, input| {
                b.iter(|| {
                    CollapsableSlice(&input[..]).collapse_frames(|frame| match frame {
                        ListFrame::Cons(e, acc) => e + acc,
                        ListFrame::Nil => 0,
                    })
                })
            },
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
