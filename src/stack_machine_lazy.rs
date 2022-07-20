use crate::functor::Functor;

pub fn unfold_and_fold_result<
    // F, a type parameter of kind * -> * that cannot be represented in rust
    E,
    Seed,
    Out,
    GenerateExpr: Functor<(), Unwrapped = Seed, To = U>, // F<Seed>
    ConsumeExpr,                                         // F<Out>
    U: Functor<Out, To = ConsumeExpr, Unwrapped = ()>,   // F<U>
    Alg: FnMut(ConsumeExpr) -> Result<Out, E>,           // F<Out> -> Result<Out, E>
    CoAlg: Fn(Seed) -> Result<GenerateExpr, E>,          // Seed -> Result<F<Seed>, E>
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

                let node = node.fmap(|seed| topush.push(State::PreVisit(seed)));

                todo.push(State::PostVisit(node));
                todo.extend(topush.into_iter());
            }
            State::PostVisit(node) => {
                let node = node.fmap(|_: ()| vals.pop().unwrap());
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
    GenerateExpr: Functor<(), Unwrapped = Seed, To = U>,
    U: Functor<Out, To = ConsumeExpr, Unwrapped = ()>,
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
                let node = node.fmap(|seed| topush.push(State::PreVisit(seed)));

                todo.push(State::PostVisit(node));
                todo.extend(topush.into_iter());
            }
            State::PostVisit(node) => {
                let node = node.fmap(|_: ()| vals.pop().unwrap());
                vals.push(alg(node))
            }
        };
    }
    vals.pop().unwrap()
}
