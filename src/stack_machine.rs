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
pub enum VizNode<Recurse> {
    Seed { txt: String },
    Out { txt: String },
    Node { txt: String, children: Vec<Recurse> },
}

#[derive(Debug, Clone)]
pub struct VizTree(Box<VizNode<VizTree>>);

impl<A, B> MapLayer<B> for VizNode<A> {
    type To = VizNode<B>;
    type Unwrapped = A;

    #[inline(always)]
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
        match self {
            VizNode::Seed { txt } => VizNode::Seed { txt },
            VizNode::Out { txt } => VizNode::Out { txt },
            VizNode::Node { txt, children } => VizNode::Node {
                txt,
                children: children.into_iter().map(f).collect(),
            },
        }
    }
}

// use serde_json::Result;

pub fn serialize_json(elems: Vec<VizTree>) -> serde_json::Result<String> {
    use serde_json::value::Value;
    let js_vals: Vec<Value> = elems
        .into_iter()
        .map(|elem| {
            expand_and_collapse(
                elem,
                |elem| *elem.0,
                |node| match node {
                    VizNode::Seed { txt } => {
                        let mut h = serde_json::Map::new();
                        h.insert("typ".to_string(), Value::String("seed".to_string()));
                        h.insert("txt".to_string(), Value::String(txt));
                        Value::Object(h)
                    }
                    VizNode::Out { txt } => {
                        let mut h = serde_json::Map::new();
                        h.insert("typ".to_string(), Value::String("out".to_string()));
                        h.insert("txt".to_string(), Value::String(txt));
                        Value::Object(h)
                    }
                    VizNode::Node { txt, children } => {
                        let mut h = serde_json::Map::new();
                        h.insert("typ".to_string(), Value::String("node".to_string()));
                        h.insert("txt".to_string(), Value::String(txt));
                        h.insert("children".to_string(), Value::Array(children));
                        Value::Object(h)
                    }
                },
            )
        })
        .collect();
    serde_json::to_string(&js_vals)
}

use std::fmt::Debug;

pub fn expand_and_collapse_v<Seed, Out, Expandable, Collapsable>(
    seed: Seed,
    mut coalg: impl FnMut(Seed) -> Expandable,
    mut alg: impl FnMut(Collapsable) -> Out,
) -> (Out, Vec<VizTree>)
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

    let mut v = vec![VizTree(Box::new(VizNode::Seed {
        txt: format!("{:?}", seed),
    }))];

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
                    if let State::Done(x) = state.remove(&idx).unwrap() {
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

        let stage_viz = {
            let t: VizTree = expand_and_collapse(
                0,
                |idx| match state.get(&idx).unwrap() {
                    State::PreVisit(seed) => VizNode::Seed {
                        txt: format!("{:?}", seed),
                    },
                    State::PostVisit(node) => {
                        let node: <Expandable as MapLayer<usize>>::To = node.clone();
                        let txt = format!("{:?}", node);
                        let mut children = Vec::new();

                        node.map_layer(|k| {
                            children.push(k);
                        });

                        VizNode::Node { txt, children }
                    }
                    State::Done(out) => VizNode::Out {
                        txt: format!("{:?}", out),
                    },
                },
                |n| VizTree(Box::new(n)),
            );

            t
        };

        v.push(stage_viz);
    }

    let idx = vals.pop().unwrap();
    if let State::Done(x) = state.remove(&idx).unwrap() {
        (x.clone(), v)
    } else {
        unreachable!()
    }
}
