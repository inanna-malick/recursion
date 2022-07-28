use crate::{
    map_layer::{CoProject, MapLayer, Project},
    Collapse, Expand,
};

impl<
        // F, a type parameter of kind * -> * that cannot be represented in rust
        Seed: Project<To = GenerateExpr>,
        Out,
        GenerateExpr: MapLayer<(), Unwrapped = Seed, To = U>, // F<Seed>
        ConsumeExpr,                                          // F<Out>
        U: MapLayer<Out, To = ConsumeExpr, Unwrapped = ()>,   // F<()>
    > Collapse<Out, ConsumeExpr> for Seed
{
    fn collapse_layers<F: FnMut(ConsumeExpr) -> Out>(self, collapse_layer: F) -> Out {
        unfold_and_fold(self, Project::project, collapse_layer)
    }
}

impl<
        // F, a type parameter of kind * -> * that cannot be represented in rust
        Seed: Project<To = GenerateExpr>,
        Out: CoProject<From = ConsumeExpr>,
        GenerateExpr: MapLayer<(), Unwrapped = Seed, To = U>, // F<Seed>
        ConsumeExpr,                                          // F<Out>
        U: MapLayer<Out, To = ConsumeExpr, Unwrapped = ()>,   // F<()>
    > Expand<Seed, GenerateExpr> for Out
{
    fn expand_layers<F: Fn(Seed) -> GenerateExpr>(seed: Seed, expand_layer: F) -> Self {
        unfold_and_fold(seed, expand_layer, CoProject::coproject)
    }
}

// NOTE: can impl recursive over _some seed value_ eg BoxExpr
// given a _project_ trait to handle the mechanical 'ana' bit
pub fn unfold_and_fold_result<
    // F, a type parameter of kind * -> * that cannot be represented in rust
    E,
    Seed,
    Out,
    GenerateExpr: MapLayer<(), Unwrapped = Seed, To = U>, // F<Seed>
    ConsumeExpr,                                          // F<Out>
    U: MapLayer<Out, To = ConsumeExpr, Unwrapped = ()>,   // F<U>
    Alg: FnMut(ConsumeExpr) -> Result<Out, E>,            // F<Out> -> Result<Out, E>
    CoAlg: Fn(Seed) -> Result<GenerateExpr, E>,           // Seed -> Result<F<Seed>, E>
>(
    seed: Seed,
    coalg: CoAlg, // Seed -> F<Seed>
    mut alg: Alg, // F<Out> -> Out
) -> Result<Out, E> {
    enum State<Pre, Post> {
        PreVisit(Pre),
        PostVisit(Post),
    }

    let mut vals: Vec<Out> = vec![];
    let mut todo: Vec<State<Seed, U>> = vec![State::PreVisit(seed)];

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

pub fn unfold_and_fold<Seed, Out, GenerateExpr, ConsumeExpr, U, Alg, CoAlg>(
    seed: Seed,
    coalg: CoAlg, // Seed   -> F<Seed>
    mut alg: Alg, // F<Out> -> Out
) -> Out
where
    GenerateExpr: MapLayer<(), Unwrapped = Seed, To = U>,
    U: MapLayer<Out, To = ConsumeExpr, Unwrapped = ()>,
    Alg: FnMut(ConsumeExpr) -> Out,
    CoAlg: Fn(Seed) -> GenerateExpr,
{
    enum State<Pre, Post> {
        PreVisit(Pre),
        PostVisit(Post),
    }

    let mut vals: Vec<Out> = vec![];
    let mut todo: Vec<State<_, _>> = vec![State::PreVisit(seed)];

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

pub fn unfold_and_fold_annotate_result<
    E,
    Seed,
    Out,
    Annotation,
    GenerateExpr,
    ConsumeExpr,
    AnnotateExpr,
    U1,
    Alg,
    Annotate,
    CoAlg,
>(
    seed: Seed,
    coalg: CoAlg,           // Seed   -> F<Seed>
    mut annotate: Annotate, // F<Annotation> -> Annotation
    mut alg: Alg,           // F<(Annotation, Out)> -> Out
) -> Result<Out, E>
where
    Annotation: Clone,
    GenerateExpr: MapLayer<(), Unwrapped = Seed, To = U1>,
    U1: Clone,
    U1: MapLayer<Annotation, To = AnnotateExpr, Unwrapped = ()>,
    U1: MapLayer<Out, To = ConsumeExpr, Unwrapped = ()>,
    Alg: FnMut(Annotation, ConsumeExpr) -> Result<Out, E>,
    Annotate: FnMut(AnnotateExpr) -> Result<Annotation, E>,
    CoAlg: Fn(Seed) -> Result<GenerateExpr, E>,
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
