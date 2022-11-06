#[cfg(any(test, feature = "experimental"))]
pub mod experimental;
#[cfg(feature = "experimental")]
pub mod visualize;

use crate::{
    map_layer::{MapLayer, Project},
    Collapse,
};

/// Build a state machine by simultaneously expanding a seed into some structure and consuming that structure from the leaves down.
/// Uses 'Result' to handle early termination

/// Type parameter explanation:
/// Layer: some partially applied type, eg Option or Vec. Not yet representable in Rust.
/// Seed: the initial value that structure is expanded out from
/// Out: the value that the structure is collapsed into
/// Expandable: a single layer of expanding structure, of type Layer<Seed>
/// Collapsable: a single layer of collapsing structure, of type Layer<Out>
/// E: a failure case that results in early termination when encountered
pub fn expand_and_collapse_result<Seed, Out, Expandable, Collapsable, Error>(
    seed: Seed,
    mut coalg: impl FnMut(Seed) -> Result<Expandable, Error>,
    mut alg: impl FnMut(Collapsable) -> Result<Out, Error>,
) -> Result<Out, Error>
where
    Expandable: MapLayer<Unwrapped = Seed>,
    <Expandable as MapLayer>::Layer<()>: MapLayer<Unwrapped = (), Layer<Out> = Collapsable>,
{
    enum State<Pre, Post> {
        PreVisit(Pre),
        PostVisit(Post),
    }

    let mut vals: Vec<Out> = vec![];
    let mut todo: Vec<State<Seed, _>> = vec![State::PreVisit(seed)];

    while let Some(item) = todo.pop() {
        match item {
            State::PreVisit(seed) => {
                let node = coalg(seed)?;
                let mut topush = Vec::new();

                let node = node.map_layer(|seed| topush.push(State::PreVisit(seed)));

                todo.push(State::PostVisit(node));
                todo.extend(topush.into_iter());
            }
            State::PostVisit(node) => {
                let node = node.map_layer(|_: ()| vals.pop().unwrap());
                vals.push(alg(node)?)
            }
        };
    }
    Ok(vals.pop().unwrap())
}

/// Build a state machine by simultaneously expanding a seed into some structure and consuming that structure from the leaves down

/// Type parameter explanation:
/// Layer: some partially applied type, eg Option or Vec. Not yet representable in Rust.
/// Seed: the initial value that structure is expanded out from
/// Out: the value that the structure is collapsed into
/// Expandable: a single layer of expanding structure, of type Layer<Seed>
/// Collapsable: a single layer of collapsing structure, of type Layer<Out>
pub fn expand_and_collapse<Seed, Out, Expandable, Collapsable>(
    seed: Seed,
    mut expand_layer: impl FnMut(Seed) -> Expandable,
    mut collapse_layer: impl FnMut(Collapsable) -> Out,
) -> Out
where
    Expandable: MapLayer<Unwrapped = Seed>,
    <Expandable as MapLayer>::Layer<()>: MapLayer<Unwrapped = (), Layer<Out> = Collapsable>,
{
    enum State<Seed, CollapsableInternal> {
        Expand(Seed),
        Collapse(CollapsableInternal),
    }

    let mut vals: Vec<Out> = vec![];
    let mut stack = vec![State::Expand(seed)];

    while let Some(item) = stack.pop() {
        match item {
            State::Expand(seed) => {
                let node = expand_layer(seed);
                let mut seeds = Vec::new();
                let node = node.map_layer(|seed| seeds.push(seed));

                stack.push(State::Collapse(node));
                stack.extend(seeds.into_iter().map(State::Expand));
            }
            State::Collapse(node) => {
                let node = node.map_layer(|_: ()| vals.pop().unwrap());
                vals.push(collapse_layer(node))
            }
        };
    }
    vals.pop().unwrap()
}

impl<
        // Layer, a type parameter of kind * -> * that cannot be represented in rust
        Seed: Project<To = GenerateExpr>,
        Out,
        GenerateExpr: MapLayer<Unwrapped = Seed>, // Layer<Seed>
        ConsumeExpr,                              // Layer<Out>
    > Collapse<Out, ConsumeExpr> for Seed
where
    <GenerateExpr as MapLayer>::Layer<()>: MapLayer<Unwrapped = (), Layer<Out> = ConsumeExpr>,
{
    fn collapse_layers<F: FnMut(ConsumeExpr) -> Out>(self, collapse_layer: F) -> Out {
        expand_and_collapse(self, Project::project, collapse_layer)
    }
}
