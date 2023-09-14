
// TODO: Docstring everything here
pub trait Recursive
where
    Self: Sized,
{
    type Frame<X>;

    fn map_frame<A, B>(input: Self::Frame<A>, f: impl FnMut(A) -> B) -> Self::Frame<B>;

    fn into_frame(self) -> Self::Frame<Self>;
}

pub trait RecursiveExt: Recursive {
    fn fold_recursive<Out>(self, collapse_layer: impl FnMut(Self::Frame<Out>) -> Out) -> Out;
}

impl<X> RecursiveExt for X
where
    X: Recursive,
{
    fn fold_recursive<Out>(
        self,
        mut collapse_layer: impl FnMut(X::Frame<Out>) -> Out,
    ) -> Out {
        enum State<Seed, CollapsableInternal> {
            Expand(Seed),
            Collapse(CollapsableInternal),
        }

        let mut vals: Vec<Out> = vec![];
        let mut stack = vec![State::Expand(self)];

        while let Some(item) = stack.pop() {
            match item {
                State::Expand(seed) => {
                    let node = Self::into_frame(seed);
                    let mut seeds = Vec::new();
                    let node = Self::map_frame(node, |seed| seeds.push(seed));

                    stack.push(State::Collapse(node));
                    stack.extend(seeds.into_iter().map(State::Expand));
                }
                State::Collapse(node) => {
                    let node = Self::map_frame(node, |_: ()| vals.pop().unwrap());
                    vals.push(collapse_layer(node))
                }
            };
        }
        vals.pop().unwrap()
    }

}
