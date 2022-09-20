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

type VizNodeId = u32;

pub enum VizAction {
    // expand a seed to a node, with new child seeds if any
    ExpandSeed {
        target_id: VizNodeId,
        txt: String,
        seeds: Vec<(VizNodeId, String)>,
    },
    // collapse node to value, removing all child nodes
    CollapseNode {
        target_id: VizNodeId,
        txt: String,
    },
}

pub struct Viz {
    seed_txt: String,
    root_id: VizNodeId,
    actions: Vec<VizAction>,
}

pub fn serialize_json(v: Viz) -> serde_json::Result<String> {
    use serde_json::value::Value;
    let actions: Vec<Value> = v
        .actions
        .into_iter()
        .map(|elem| match elem {
            VizAction::ExpandSeed {
                target_id,
                txt,
                seeds,
            } => {
                let mut h = serde_json::Map::new();
                h.insert(
                    "target_id".to_string(),
                    Value::String(target_id.to_string()),
                );
                h.insert("txt".to_string(), Value::String(txt));
                let mut json_seeds = Vec::new();
                for (node_id, txt) in seeds.into_iter() {
                    let mut h = serde_json::Map::new();
                    h.insert("node_id".to_string(), Value::String(node_id.to_string()));
                    h.insert("txt".to_string(), Value::String(txt));
                    json_seeds.push(Value::Object(h));
                }
                h.insert("seeds".to_string(), Value::Array(json_seeds));
                Value::Object(h)
            }
            VizAction::CollapseNode { target_id, txt } => {
                let mut h = serde_json::Map::new();
                h.insert(
                    "target_id".to_string(),
                    Value::String(target_id.to_string()),
                );
                h.insert("txt".to_string(), Value::String(txt));
                Value::Object(h)
            }
        })
        .collect();

    let viz_root = {
        let mut h = serde_json::Map::new();
        h.insert("node_id".to_string(), Value::String(v.root_id.to_string()));
        h.insert("txt".to_string(), Value::String(v.seed_txt));
        h.insert("typ".to_string(), Value::String("seed".to_string()));
        Value::Object(h)
    };

    let viz_js = {
        let mut h = serde_json::Map::new();
        h.insert("root".to_string(), viz_root);
        h.insert("actions".to_string(), Value::Array(actions));
        Value::Object(h)
    };

    serde_json::to_string(&viz_js)
}

// use std::fmt::Debug;
// TODO: split out root seed case to separate field on return obj, not needed as part of enum!
pub fn expand_and_collapse_v<Seed, Out, Expandable, Collapsable>(
    seed: Seed,
    mut coalg: impl FnMut(Seed) -> Expandable,
    mut alg: impl FnMut(Collapsable) -> Out,
) -> (Out, Viz)
where
    Expandable: MapLayer<(), Unwrapped = Seed>,
    <Expandable as MapLayer<()>>::To:
        MapLayer<Out, Unwrapped = (), To = Collapsable> + std::fmt::Debug,
    Seed: std::fmt::Debug,
    Out: std::fmt::Debug,
{
    enum State<Pre, Post> {
        PreVisit(Pre),
        PostVisit(Post),
    }

    let mut keygen = 1; // 0 is used for root node
    let mut v = Vec::new();
    let root_seed_txt = format!("{:?}", seed);

    let mut vals: Vec<Out> = vec![];
    let mut todo: Vec<State<(VizNodeId, Seed), _>> = vec![State::PreVisit((0, seed))];

    while let Some(item) = todo.pop() {
        match item {
            State::PreVisit((viz_node_id, seed)) => {
                let mut seeds_v = Vec::new();

                let node = coalg(seed);
                let mut topush = Vec::new();
                let node = node.map_layer(|seed| {
                    let k = keygen;
                    keygen += 1;
                    seeds_v.push((k, format!("{:?}", seed)));

                    topush.push(State::PreVisit((k, seed)))
                });

                v.push(VizAction::ExpandSeed {
                    target_id: viz_node_id,
                    txt: format!("{:?}", node),
                    seeds: seeds_v,
                });

                todo.push(State::PostVisit((viz_node_id, node)));
                todo.extend(topush.into_iter());
            }
            State::PostVisit((viz_node_id, node)) => {
                let node = node.map_layer(|_: ()| vals.pop().unwrap());

                let out = alg(node);

                v.push(VizAction::CollapseNode {
                    target_id: viz_node_id,
                    txt: format!("{:?}", out),
                });

                vals.push(out)
            }
        };
    }
    (vals.pop().unwrap(), Viz{ seed_txt: root_seed_txt, root_id: 0, actions: v })
}
