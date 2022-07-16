use crate::{
    functor::Functor,
    recursive_traits::{CoRecursive, Recursive},
};

/// Generic struct used to represent a recursive structure of some type F<usize>
pub struct RecursiveStruct<F> {
    // nonempty, in topological-sorted order
    elems: Vec<F>,
}

impl<'a, F> RecursiveStruct<F> {
    pub fn as_ref(&'a self) -> RecursiveStructRef<'a, F> {
        RecursiveStructRef {
            elems: &self.elems[..],
        }
    }
}

/// Generic struct used to represent a refernce to a recursive structure of some type F<usize>
pub struct RecursiveStructRef<'a, F> {
    // nonempty, in topological-sorted order
    elems: &'a [F],
}

// HOLY SHIT: if I build this with a DFS I can use, like, a simple stack to keep track of things
//            like, each eval phase just pops some elements, EXACT OPPOSITE ARROWS OF CONSTRUCTION
// haha nice nice nice nice nice - will just need to change impl here to push and keep it working,
// can impl pop-based situation next. wait, holy shit, if it just runs pop I can have a vec of Expr<()>
// NOTE: adds hard requirement, functor traversal order MUST be constant. woah.
impl<A, U, O: Functor<(), Unwrapped = A, To = U>> CoRecursive<A, O> for RecursiveStruct<U> {
    fn unfold<F: Fn(A) -> O>(a: A, coalg: F) -> Self {
        let mut frontier = Vec::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop() {
            let node = coalg(seed);

            let mut topush = Vec::new();

            let node = node.fmap(|aa| {
                topush.push(aa);
                ()
            });

            frontier.extend(topush.into_iter().rev());

            elems.push(node);
        }

        elems.reverse();

        Self { elems }
    }
}

impl<A, O, U: Functor<A, To = O, Unwrapped = ()>> Recursive<A, O> for RecursiveStruct<U> {
    fn fold<F: FnMut(O) -> A>(self, mut alg: F) -> A {
        let mut result_stack = Vec::new();

        for node in self.elems.into_iter() {
            let alg_res = {
                // each node is only referenced once so just remove it, also we know it's there so unsafe is fine
                let node = node.fmap(|_| result_stack.pop().unwrap());

                alg(node)
            };
            result_stack.push(alg_res);
        }

        result_stack.pop().unwrap()
    }
}


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
    coalg: CoAlg,
    mut alg: Alg,
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

                let node = node.fmap(|seed| {
                    topush.push(State::PreVisit(seed));
                    ()
                });

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

pub fn unfold_and_fold<
    Seed,
    Out,
    GenerateExpr: Functor<(), Unwrapped = Seed, To = U>,
    ConsumeExpr,
    U: Functor<Out, To = ConsumeExpr, Unwrapped = ()>,
    Alg: FnMut(ConsumeExpr) -> Out,
    CoAlg: Fn(Seed) -> GenerateExpr,
>(
    seed: Seed,
    coalg: CoAlg,
    mut alg: Alg,
) -> Out {
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

                let node = node.fmap(|seed| {
                    topush.push(State::PreVisit(seed));
                    ()
                });

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

// TODO: consider using slab instead of vec for underlying RecursiveStruct

// TODO: use noop hasher impl for usize - much much faster, all usizes are unique

// IDEA - take a mutable ref - &mut self, for fold and unfold - could then use vec drain (?) - so then struct is holding ARENA instead of just the elem- 'recursion scheme evaluator type' - could own and reuse hashmap
// IDEA (cont) - if I'm repeatedly evaluating a cata I could reuse an arena? would have to grow it for each eval - can drop arena each eval and reuse allocation, can amortize to LESS THAN ONE EVAL per fold
// would use same alloc for fold/unfolds - evaluator struct tied to single <A, O>

// can use slab to impl fused fold/unfold mb - also read impl? it's interesting

// TODO - compile pass over F<slabref> to preserve recursive links

impl<'a, A: std::fmt::Debug, O: 'a, U: std::fmt::Debug> Recursive<A, O>
    for RecursiveStructRef<'a, U>
where
    &'a U: Functor<A, To = O, Unwrapped = ()>,
{
    fn fold<F: FnMut(O) -> A>(self, mut alg: F) -> A {
        let mut result_stack = Vec::with_capacity(32);

        for node in self.elems.iter() {
            let node = node.fmap(|_| unsafe { result_stack.pop().unwrap_unchecked() });

            result_stack.push(alg(node));
        }

        unsafe { result_stack.pop().unwrap_unchecked() }
    }
}
