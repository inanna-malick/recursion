/// A single 'frame' containing values that can be mapped over via `map_frame`.
///
/// This trait is usually implemented for some marker token, because rust does not
/// allow for implementing a trait for a partially applied type.
///
/// For this reason, a common convention is to implement this trait using the uninhabited
///  `PartiallyApplied` type, eg
/// ```rust
/// use recursion_schemes::{MappableFrame, PartiallyApplied};
///
/// enum IntTreeFrame<A> {
///     Leaf { value: usize },
///     Node { left: A, right: A },
/// }
///
/// impl MappableFrame for IntTreeFrame<PartiallyApplied> {
///     type Frame<X> = IntTreeFrame<X>;
///
///     fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
///         match input {
///             IntTreeFrame::Leaf { value } => IntTreeFrame::Leaf { value },
///             IntTreeFrame::Node { left, right } => IntTreeFrame::Node {
///                 left: f(left),
///                 right: f(right),
///             },
///         }
///     }
/// }
/// ```
pub trait MappableFrame {
    /// the frame type that is mapped over by `map_frame`
    type Frame<X>;

    /// Apply some function `f` to each element inside a frame
    fn map_frame<A, B>(input: Self::Frame<A>, f: impl FnMut(A) -> B) -> Self::Frame<B>;
}

/// `PartiallyApplied` is an uninhabited enum - a type that cannot exist at runtime.
/// It is used to defined MappableFrame instances for partially-applied types.
///
/// For example: the MappableFrame instance for `MyFrame<A>` cannot be written over the
/// partially-applied type `MyFrame`, so instead we write it over `MyFrame<PartiallyApplied>`
#[derive(Debug)]
pub enum PartiallyApplied {}

/// This function generates a stack machine for some frame `F::Frame`,
/// expanding some seed value `Seed` into frames via a function `Seed -> Frame<Seed>`
/// and collapsing those values via a function `Frame<Out> -> Out`.
///
/// This function performs a depth-first traversal, expanding and collapsing each branch in turn
///
/// This function is stack safe (it does not use the call stack), but it
/// does use an internal stack data structure and is thus, technically,
/// susceptible to stack overflows if said stack expands
pub fn expand_and_collapse<F: MappableFrame, Seed, Out>(
    seed: Seed,
    mut expand_frame: impl FnMut(Seed) -> F::Frame<Seed>,
    mut collapse_frame: impl FnMut(F::Frame<Out>) -> Out,
) -> Out {
    enum State<Seed, CollapsableInternal> {
        Expand(usize, Seed),
        Collapse(usize, CollapsableInternal),
    }

    let mut vals: Vec<Option<Out>> = vec![None];
    let mut stack = vec![State::Expand(0, seed)];

    while let Some(item) = stack.pop() {
        match item {
            State::Expand(val_idx, seed) => {
                let node = expand_frame(seed);
                let mut seeds = Vec::new();
                let node = F::map_frame(node, |seed| {
                    vals.push(None);
                    let idx = vals.len() - 1;
                    seeds.push(State::Expand(idx, seed));
                    idx
                });

                stack.push(State::Collapse(val_idx, node));
                stack.extend(seeds);
            }
            State::Collapse(val_idx, node) => {
                let node = F::map_frame(node, |k| vals[k].take().unwrap());
                vals[val_idx] = Some(collapse_frame(node));
            }
        };
    }
    vals[0].take().unwrap()
}
