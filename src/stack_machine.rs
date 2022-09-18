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
    Seed {
        id: usize,
        txt: String,
    },
    Out {
        id: usize,
        txt: String,
    },
    Node {
        id: usize,
        txt: String,
        children: Vec<Recurse>,
    },
}

#[derive(Debug, Clone)]
pub struct VizTree(Box<VizNode<VizTree>>);

impl<A, B> MapLayer<B> for VizNode<A> {
    type To = VizNode<B>;
    type Unwrapped = A;

    #[inline(always)]
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
        match self {
            VizNode::Seed { id, txt } => VizNode::Seed { txt, id },
            VizNode::Out { id, txt } => VizNode::Out { txt, id },
            VizNode::Node { id, txt, children } => VizNode::Node {
                id,
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
                    VizNode::Seed { id, txt } => {
                        let mut h = serde_json::Map::new();
                        h.insert("id".to_string(), Value::String(id.to_string()));
                        // todo: to_expand
                        h.insert("typ".to_string(), Value::String("seed".to_string()));
                        h.insert("txt".to_string(), Value::String(txt));
                        Value::Object(h)
                    }
                    VizNode::Out { id, txt } => {
                        let mut h = serde_json::Map::new();
                        h.insert("id".to_string(), Value::String(id.to_string()));
                        h.insert("typ".to_string(), Value::String("out".to_string()));
                        h.insert("txt".to_string(), Value::String(txt));
                        Value::Object(h)
                    }
                    VizNode::Node { id, txt, children } => {
                        let mut h = serde_json::Map::new();
                        // todo: collapsed into:
                        h.insert("id".to_string(), Value::String(id.to_string()));
                        h.insert("typ".to_string(), Value::String("structure".to_string()));
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


// ok. fucking christ. d3 sucks. how to do this?
// answer: return one tree, w/ every node in the state.
// idea is to: have state transition info on each node, such that we can run node.each to update the tree each time
// there are a series of animation frames
// each node (except the root?) starts hidden - impl'd in d3 itself, hidden via attr on node (or! child node info, idk)
// yes, this - set some hide attr on the node to, well, hide. done, lol. nice.
// each node in the output tree has a sparse map: eg {"1": [seed(name), structure(name), collapse(..)]}
// so, like, std lifecycle: seed(..) -> structure(..) -> collapse(..), and the frame # is used to trigger update
//   do the foreach and update node name/typ based on frame map lookup (if any)
//   ok, what's weird - there's no change in tree structure here I think, just visibility. actually works well for me I think!
//   # of children for any node actually doesn't change, just visibility lol hell yes
//  this will be a bit of a hassle to generate, but it'll be more compact and also not in JS lol
//  ok, quick test - this works (simple flag, switching based on bool negation), can expand out if I pack in the data.
//  NOTE: seed is the one that adds a node at all, but we also need 'remove' for when the parent of a node is set to collapes
//            NOTE: or do we! b/c when a node is set to collapse, (previously structure), then we can zero out node children
//            NOTE: shit yeah it still does, b/c then we need to 
//            NOTE: actually, could even just show unexpanded nodes as greyed out - dotted lines or similar
pub fn expand_and_collapse_v<Seed, Out, Expandable, Collapsable>(
    seed: Seed,
    mut coalg: impl FnMut(Seed) -> Expandable,
    mut alg: impl FnMut(Collapsable) -> Out,
) -> (Out, Vec<VizTree>)
where
    Seed: Debug,
    Out: Debug,
    Expandable: MapLayer<usize, Unwrapped = Seed>,
    <Expandable as MapLayer<usize>>::To: MapLayer<Out, Unwrapped = usize, To = Collapsable>
        + Debug
        + Clone
        + MapLayer<(), Unwrapped = usize>,
    <<Expandable as MapLayer<usize>>::To as MapLayer<()>>::To: Debug,
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
        id: 0,
        txt: format!("{:?}", seed),
    }))];

    state.insert(0, State::PreVisit(seed));

    let mut vals: Vec<StateIdx> = vec![];
    let mut todo: Vec<StateIdx> = vec![0];

    while let Some(idx) = todo.pop() {
        let node = state.remove(&idx).unwrap();
        match node {
            State::PreVisit(seed) => {
                let node = coalg(seed);
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
                        x
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
                        id: idx,
                        txt: format!("{:?}", seed),
                    },
                    State::PostVisit(node) => {
                        let node: <Expandable as MapLayer<usize>>::To = node.clone();
                        let mut children = Vec::new();

                        let node = node.map_layer(|k| {
                            children.push(k);
                        });

                        let txt = format!("{:?}", node);

                        VizNode::Node {
                            id: idx,
                            txt,
                            children,
                        }
                    }
                    State::Done(out) => VizNode::Out {
                        id: idx,
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
        (x, v)
    } else {
        unreachable!()
    }
}
