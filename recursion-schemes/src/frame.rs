/// A single 'frame' in a recursive structure. For example: `enum ExprFrame<A> { Literal(u32), Add(A, A), Mul(A, A)}`
/// represents a single frame of an expression tree with literal integers, addition, and multiplication. The expression
/// "1 + 2 * 3" could be represented using ExprFrame::Add, ExprFrame::Literal(1), etc
pub trait MappableFrame {
    type Frame<X>;

    fn map_frame<A, B>(input: Self::Frame<A>, f: impl FnMut(A) -> B) -> Self::Frame<B>;
}

pub trait MappableFrameExt: MappableFrame {
    fn expand_and_collapse<Seed, Out>(
        seed: Seed,
        expand_frame: impl FnMut(Seed) -> <Self as MappableFrame>::Frame<Seed>,
        collapse_frame: impl FnMut(<Self as MappableFrame>::Frame<Out>) -> Out,
    ) -> Out;
}

impl<X> MappableFrameExt for X
where
    X: MappableFrame,
{
    fn expand_and_collapse<Seed, Out>(
        seed: Seed,
        mut expand_frame: impl FnMut(Seed) -> <X as MappableFrame>::Frame<Seed>,
        mut collapse_frame: impl FnMut(<X as MappableFrame>::Frame<Out>) -> Out,
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
                    let node = Self::map_frame(node, |seed| seeds.push(seed));

                    stack.push(State::Collapse(node));
                    stack.extend(seeds.into_iter().map(State::Expand));
                }
                State::Collapse(node) => {
                    let node = Self::map_frame(node, |_: ()| vals.pop().unwrap());
                    vals.push(collapse_frame(node))
                }
            };
        }
        vals.pop().unwrap()
    }
}
