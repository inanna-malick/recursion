#[cfg(any(test, feature = "experimental"))]
pub mod experimental;
// #[cfg(any(test, feature = "experimental"))]
// pub mod visualize;

use std::collections::HashMap;

use crate::map_layer::MapLayer;

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
    Expandable: MapLayer<(), Unwrapped = Seed>,
    <Expandable as MapLayer<()>>::To: MapLayer<Out, Unwrapped = (), To = Collapsable>,
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
    mut coalg: impl FnMut(Seed) -> Expandable,
    mut alg: impl FnMut(Collapsable) -> Out,
) -> Out
where
    Expandable: MapLayer<(), Unwrapped = Seed>,
    <Expandable as MapLayer<()>>::To: MapLayer<Out, Unwrapped = (), To = Collapsable>,
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
                let node = coalg(seed);
                let mut topush = Vec::new();
                let node = node.map_layer(|seed| topush.push(State::PreVisit(seed)));

                todo.push(State::PostVisit(node));
                todo.extend(topush.into_iter());
            }
            State::PostVisit(node) => {
                let node = node.map_layer(|_: ()| vals.pop().unwrap());
                vals.push(alg(node))
            }
        };
    }
    vals.pop().unwrap()
}

pub type NodeIdx = usize;

#[derive(Debug, Clone)]
pub enum VizNode {
    Seed(String),
    Out(String),
    Node(String, Vec<NodeIdx>),
}

#[derive(Debug, Clone)]
pub struct Viz {
    root: NodeIdx,
    nodes: HashMap<NodeIdx, VizNode>,
}

use std::fmt::Debug;

pub fn expand_and_collapse_v<Seed, Out, Expandable, Collapsable>(
    seed: Seed,
    mut coalg: impl FnMut(Seed) -> Expandable,
    mut alg: impl FnMut(Collapsable) -> Out,
) -> (Out, Vec<Viz>)
where
    Seed: Clone + Debug,
    Out: Clone + Debug,
    Expandable: MapLayer<usize, Unwrapped = Seed>,
    <Expandable as MapLayer<usize>>::To: MapLayer<Out, Unwrapped = usize, To = Collapsable>
        + Debug
        + Clone
        + MapLayer<(), Unwrapped = usize>,
{
    enum State<Pre, Post, Done> {
        PreVisit(Pre),
        PostVisit(Post),
        Done(Done),
    }

    let mut keygen = 1; // 0 is used for root node

    type StateIdx = usize;

    let mut state: HashMap<StateIdx, State<Seed, _, Out>> = HashMap::new();

    let mut v = vec![Viz {
        nodes: {
            let mut h = HashMap::new();
            h.insert(0, VizNode::Seed(format!("{:?}", seed)));
            h
        },
        root: 0,
    }];

    state.insert(0, State::PreVisit(seed));

    let mut vals: Vec<StateIdx> = vec![];
    let mut todo: Vec<StateIdx> = vec![0];

    while let Some(idx) = todo.pop() {
        let node = state.remove(&idx).unwrap();
        match node {
            State::PreVisit(seed) => {
                let node = coalg(seed.clone());
                let mut topush = Vec::new();
                let node = node.map_layer(|seed| {
                    let k = keygen;
                    keygen += 1;
                    state.insert(k, State::PreVisit(seed));
                    topush.push(k);
                    k
                });

                state.insert(idx, State::PostVisit(node));

                todo.push(idx);
                todo.extend(topush.into_iter());
            }
            State::PostVisit(node) => {
                let node = node.map_layer(|_idx: usize| {
                    // note idx not used here, only recorded to simplify visualization
                    let idx = vals.pop().unwrap();
                    if let State::Done(x) = state.get(&idx).unwrap() {
                        x.clone()
                    } else {
                        unreachable!()
                    }
                });

                let out = alg(node);

                state.insert(idx, State::Done(out));

                vals.push(idx)
            }
            State::Done(_) => unreachable!(),
        };

        let stage_viz = Viz {
            nodes: {
                let mut h = HashMap::new();

                for (key, node) in state.iter() {
                    h.insert(
                        key.clone(),
                        match node {
                            State::PreVisit(seed) => VizNode::Seed(format!("{:?}", seed)),
                            State::PostVisit(node) => {
                                let node: <Expandable as MapLayer<usize>>::To = node.clone();
                                let s = format!("{:?}", node);
                                let mut children = Vec::new();

                                node.map_layer(|k| {
                                    children.push(k);
                                });

                                VizNode::Node(s, children)
                            }
                            State::Done(out) => VizNode::Out(format!("{:?}", out)),
                        },
                    );
                }

                h
            },
            root: 0,
        };

        v.push(stage_viz);
    }

    let idx = vals.pop().unwrap();
    if let State::Done(x) = state.get(&idx).unwrap() {
        (x.clone(), v)
    } else {
        unreachable!()
    }
}
