/// A single 'frame' in a recursive structure. For example: `enum ExprFrame<A> { Literal(u32), Add(A, A), Mul(A, A)}`
/// represents a single frame of an expression tree with literal integers, addition, and multiplication. The expression
/// "1 + 2 * 3" could be represented using ExprFrame::Add, ExprFrame::Literal(1), etc
pub trait MappableFrame {
    type Frame<Next>;

    fn map_frame<A, B>(input: Self::Frame<A>, f: impl FnMut(A) -> B) -> Self::Frame<B>;
}

// NOTE TO FUTURE ME: this is an important insight I think, for working with borrowed data
// I still need to make sure it works in practice tho, and tbh it's worse than just defining Recursive over &'a Foo
pub trait MappableFrameRef: MappableFrame {
    type RefFrameToken<'a>: MappableFrame;

    fn as_ref<X>(
        input: &Self::Frame<X>,
    ) -> <Self::RefFrameToken<'_> as MappableFrame>::Frame<&X>;
}

// pub trait MappableFrameRef {
//     type Frame<'a, Next>;

//     fn map_frame<'a, A, B>(
//         input: &'a Self::Frame<'a, A>,
//         f: impl FnMut(A) -> B,
//     ) -> Self::Frame<'a, B>;
// }

pub fn expand_and_collapse<F: MappableFrame, Seed, Out>(
    seed: Seed,
    mut expand_frame: impl FnMut(Seed) -> F::Frame<Seed>,
    mut collapse_frame: impl FnMut(F::Frame<Out>) -> Out,
) -> Out {
    enum State<Seed, CollapsableInternal> {
        Expand(Seed),
        Collapse(CollapsableInternal),
    }

    let mut vals: Vec<Out> = vec![];
    let mut stack = vec![State::Expand(seed)];

    while let Some(item) = stack.pop() {
        match item {
            State::Expand(seed) => {
                let node = expand_frame(seed);
                let mut seeds = Vec::new();
                let node = F::map_frame(node, |seed| seeds.push(seed));

                stack.push(State::Collapse(node));
                stack.extend(seeds.into_iter().map(State::Expand));
            }
            State::Collapse(node) => {
                let node = F::map_frame(node, |_: ()| vals.pop().unwrap());
                vals.push(collapse_frame(node))
            }
        };
    }
    vals.pop().unwrap()
}

pub fn collapse_compact<F: MappableFrame, Out>(
    c: crate::recursive::Compact<F>,
    mut collapse_frame: impl FnMut(F::Frame<Out>) -> Out,
) -> Out {
    let mut vals: Vec<Out> = vec![];

    for item in c.0.into_iter() {
        let node = F::map_frame(item, |_: ()| vals.pop().unwrap());
        vals.push(collapse_frame(node))
    }
    vals.pop().unwrap()
}

pub fn collapse_compact_ref<'a, 'c: 'a, F: MappableFrameRef, Out>(
    c: &'c crate::recursive::Compact<F>,
    mut collapse_frame: impl FnMut(<F::RefFrameToken<'a> as MappableFrame>::Frame<Out>) -> Out,
) -> Out {
    let mut vals: Vec<Out> = vec![];

    for item in c.0.iter() {
        let node = <F::RefFrameToken<'a>>::map_frame(F::as_ref(item), |_: &()| vals.pop().unwrap());
        vals.push(collapse_frame(node))
    }
    vals.pop().unwrap()
}

// important note: I don't really want to do this, let's elide perf data for the GAT impl and center ergonomics
pub fn expand_compact<F: MappableFrame, Seed>(
    seed: Seed,
    mut expand_frame: impl FnMut(Seed) -> F::Frame<Seed>,
) -> crate::recursive::Compact<F> {
    let mut frontier = Vec::from([seed]);
    let mut elems = vec![];

    // expand to build a vec of elems while preserving topo order
    while let Some(seed) = frontier.pop() {
        let frame = expand_frame(seed);

        let mut topush = Vec::new();
        let frame = F::map_frame(frame, |aa| {
            topush.push(aa);
        });
        frontier.extend(topush.into_iter().rev());

        elems.push(frame);
    }

    elems.reverse();

    crate::recursive::Compact(elems)
}

// pub fn collapse_compact_ref<'a, F, Out>(
//     c: crate::recursive::CompactRef<'a, F>,
//     mut collapse_frame: impl FnMut(<F as MappableFrame>::Frame<Out>) -> Out,
// ) -> Out
// where F: MappableFrame
// {

//     let mut vals: Vec<Out> = vec![];

//     let mut iter = c.0.into_iter();

//     while let Some(item) = iter.next() {
//         let node = <F as MappableFrame>::map_frame(item, |_: ()| vals.pop().unwrap());
//         vals.push(collapse_frame(node))
//     }
//     vals.pop().unwrap()
// }
