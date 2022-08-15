use crate::{
    map_layer::{CoProject, MapLayer, Project},
    Collapse, Expand,
};

impl<
        // F, a type parameter of kind * -> * that cannot be represented in rust
        Seed: Project<To = Expandable>,
        Out,
        Expandable: MapLayer<(), Unwrapped = Seed, To = U>, // F<Seed>
        Collapsable,                                          // F<Out>
        U: MapLayer<Out, To = Collapsable, Unwrapped = ()>,   // F<()>
    > Collapse<Out, Collapsable> for Seed
{
    fn collapse_layers<F: FnMut(Collapsable) -> Out>(self, collapse_layer: F) -> Out {
        unfold_and_fold(self, Project::project, collapse_layer)
    }
}

impl<
        // F, a type parameter of kind * -> * that cannot be represented in rust
        Seed: Project<To = Expandable>,
        Out: CoProject<From = Collapsable>,
        Expandable: MapLayer<(), Unwrapped = Seed, To = U>, // F<Seed>
        Collapsable,                                          // F<Out>
        U: MapLayer<Out, To = Collapsable, Unwrapped = ()>,   // F<()>
    > Expand<Seed, Expandable> for Out
{
    fn expand_layers<F: Fn(Seed) -> Expandable>(seed: Seed, expand_layer: F) -> Self {
        unfold_and_fold(seed, expand_layer, CoProject::coproject)
    }
}

/// Build a state machine by simultaneously expanding a seed into some structure and consuming that structure from the leaves down. 
/// Uses 'Result' to handle early termination
pub fn unfold_and_fold_result<Seed, Expandable, Collapsable, Out, E>(
    seed: Seed,
    mut coalg: impl FnMut(Seed) -> Result<Expandable, E>,
    mut alg: impl FnMut(Collapsable) -> Result<Out, E>,
) -> Result<Out, E>
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
pub fn unfold_and_fold<Seed, Expandable, Collapsable, Out>(
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


#[derive(Debug, Clone, Copy)]
// shortcircuit if a given subnode of this node returns 'on', returning value 'and_return' from this node immediately
pub struct ShortCircuit<A> {
    pub on: A,
    pub rtrn: A,
}


// motivation: early termination (eg &&, either branch is true no need to eval other branch)
// since early termination logic flows from the root downwards and evaluation flows from the leaves
// up, we register the early termination logic while building the state machine and use it while collapsing it
pub fn unfold_and_fold_early_termination<Seed, Expandable, Collapsable, Out>(
    seed: Seed,
    coalg: impl Fn(Seed) -> Expandable, // Seed   -> F<(Seed, Option<ShortCircuit<Out>)>
    mut alg: impl FnMut(Collapsable) -> Out, // F<Out> -> Out
) -> Out
where
    Out: PartialEq + Eq,
    Expandable: MapLayer<(), Unwrapped = (Seed, Option<ShortCircuit<Out>>)>,
    <Expandable as MapLayer<()>>::To: MapLayer<Out, Unwrapped = (), To = Collapsable>,
{
    struct EarlyTerm<Out> {
        truncate_todo_to: usize,
        truncate_vals_to: usize,
        short_circuit: ShortCircuit<Out>,
    }

    enum State<Pre, Post, Out> {
        PreVisit {
            seed: Pre,
            early_term: Option<EarlyTerm<Out>>,
        },
        PostVisit {
            node: Post,
            early_term: Option<EarlyTerm<Out>>,
        },
    }

    let mut vals: Vec<Out> = vec![];
    let mut todo: Vec<State<_, _, Out>> = vec![State::PreVisit {
        seed,
        early_term: None,
    }];

    while let Some(item) = todo.pop() {
        match item {
            State::PreVisit { seed, early_term } => {
                let node = coalg(seed);

                let truncate_todo_to = todo.len();
                let truncate_vals_to = vals.len();

                let mut topush = Vec::new();
                let node = node.map_layer(|(seed, sc)| {
                    let early_term = sc.map(|sc| EarlyTerm {
                        truncate_todo_to,
                        truncate_vals_to,
                        short_circuit: sc,
                    });
                    topush.push(State::PreVisit { seed, early_term })
                });

                todo.push(State::PostVisit { node, early_term });
                todo.extend(topush.into_iter());
            }
            State::PostVisit {
                early_term:
                    Some(EarlyTerm {
                        truncate_todo_to,
                        truncate_vals_to,
                        short_circuit,
                    }),
                node,
            } => {
                let node = node.map_layer(|_: ()| vals.pop().unwrap());
                let res = alg(node);

                if res == short_circuit.on {
                    vals.truncate(truncate_vals_to);
                    todo.truncate(truncate_todo_to);
                    vals.push(short_circuit.rtrn);
                } else {
                    vals.push(res)
                }
            }
            State::PostVisit {
                early_term: None,
                node,
            } => {
                let node = node.map_layer(|_: ()| vals.pop().unwrap());
                vals.push(alg(node));
            }
        };
    }
    vals.pop().unwrap()
}




// TODO move to 'experimental' module or some shit



/// this function is 'spooky' and has a 'terrifying type signature'. It will likely change multiple times before being finalized
pub fn unfold_and_fold_annotate_result<
    E,
    Seed,
    Out,
    Annotation,
    Expandable,
    Collapsable,
    AnnotateExpr,
    U1,
    Alg,
    Annotate,
    CoAlg,
>(
    seed: Seed,
    mut coalg: CoAlg,       // Seed   -> F<Seed>
    mut annotate: Annotate, // F<Annotation> -> Annotation
    mut alg: Alg,           // F<(Annotation, Out)> -> Out
) -> Result<Out, E>
where
    Annotation: Clone,
    Expandable: MapLayer<(), Unwrapped = Seed, To = U1>,
    U1: Clone,
    U1: MapLayer<Annotation, To = AnnotateExpr, Unwrapped = ()>,
    U1: MapLayer<Out, To = Collapsable, Unwrapped = ()>,
    Alg: FnMut(Annotation, Collapsable) -> Result<Out, E>,
    Annotate: FnMut(AnnotateExpr) -> Result<Annotation, E>,
    CoAlg: FnMut(Seed) -> Result<Expandable, E>,
{
    enum State<Pre, Annotation, Post> {
        PreVisit(Pre),
        Annotate(Post),
        PostVisit(Annotation, Post),
    }

    let mut vals: Vec<Out> = vec![];
    let mut annotate_vals: Vec<Annotation> = vec![];
    let mut todo: Vec<State<_, Annotation, U1>> = vec![State::PreVisit(seed)];

    while let Some(item) = todo.pop() {
        match item {
            State::PreVisit(seed) => {
                let layer = coalg(seed)?;
                let mut topush = Vec::new();
                let layer: U1 = layer.map_layer(|seed| topush.push(State::PreVisit(seed)));

                todo.push(State::Annotate(layer));
                todo.extend(topush.into_iter());
            }
            State::Annotate(layer) => {
                let layer2 = layer
                    .clone()
                    .map_layer(|_: ()| annotate_vals.pop().unwrap());
                let annotation = annotate(layer2)?;
                todo.push(State::PostVisit(annotation.clone(), layer));
                annotate_vals.push(annotation);
            }
            State::PostVisit(annotation, layer) => {
                let layer = layer.map_layer(|_: ()| vals.pop().unwrap());
                vals.push(alg(annotation, layer)?)
            }
        };
    }
    Ok(vals.pop().unwrap())
}