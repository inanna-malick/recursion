use crate::db::DBKey;
use futures::future;
use futures::future::BoxFuture;
use futures::FutureExt;
use std::collections::HashMap;
use std::collections::VecDeque;

// this is the core of what users provide

#[derive(Debug, Clone, Copy)]
pub enum Expr<A> {
    Add(A, A),
    Sub(A, A),
    Mul(A, A),
    LiteralInt(i64),
    DatabaseRef(DBKey),
}

impl<A> Expr<A> {
    pub fn fmap_into<B, F: FnMut(A) -> B>(self, mut f: F) -> Expr<B> {
        match self {
            Expr::Add(a, b) => Expr::Add(f(a), f(b)),
            Expr::Sub(a, b) => Expr::Sub(f(a), f(b)),
            Expr::Mul(a, b) => Expr::Mul(f(a), f(b)),
            Expr::LiteralInt(x) => Expr::LiteralInt(x),
            Expr::DatabaseRef(x) => Expr::DatabaseRef(x),
        }
    }
}


trait MapToUsize {
    type Unwrapped;
    type To;
    fn fmap_into_usize<F: FnMut(Self::Unwrapped) -> usize>(self, f: F) -> Self::To;
}

impl<A> MapToUsize for Expr<A> {
    type To = Expr<usize>;
    type Unwrapped = A;
    fn fmap_into_usize<F: FnMut(Self::Unwrapped) -> usize>(self, f: F) -> Self::To {
        self.fmap_into(f)
    }
}

trait MapFromUsize {
    type Unwrapped;
    type From;
    fn fmap_from_usize<F: FnMut(usize) -> Self::Unwrapped>(from: Self::From, f: F) -> Self;
}

impl<A> MapFromUsize for Expr<A> {
    type From = Expr<usize>;
    type Unwrapped = A;
    fn fmap_from_usize<F: FnMut(usize) -> Self::Unwrapped>(from: Self::From, f: F) -> Self {
        from.fmap_into(f)
    }
}



trait TryJoinFuture<'a> {
    type Output;
    type Error;
    fn try_join(self) -> BoxFuture<'a, Result<Self::Output, Self::Error>>;
}

impl<'a, A: 'a + Send, E: 'a> TryJoinFuture<'a> for Expr<BoxFuture<'a, Result<A, E>>> {
    type Output = Expr<A>;
    type Error = E;

    fn try_join(self) -> BoxFuture<'a, Result<Self::Output, Self::Error>> {
        try_join_helper(self).boxed()
    }
}

async fn try_join_helper<A, E>(e: Expr<BoxFuture<'_, Result<A, E>>>) -> Result<Expr<A>, E> {
    match e {
        Expr::Add(a, b) => {
            let (a, b) = future::try_join(a, b).await?;
            Ok(Expr::Add(a, b))
        }
        Expr::Sub(a, b) => {
            let (a, b) = future::try_join(a, b).await?;
            Ok(Expr::Sub(a, b))
        }

        Expr::Mul(a, b) => {
            let (a, b) = future::try_join(a, b).await?;
            Ok(Expr::Mul(a, b))
        }

        Expr::LiteralInt(x) => Ok(Expr::LiteralInt(x)),
        Expr::DatabaseRef(key) => Ok(Expr::DatabaseRef(key)),
    }
}

pub trait Recursive<A> {
    type AlgFrom;
    type AlgTo;
    fn cata<F: FnMut(Self::AlgFrom) -> Self::AlgTo>(self, alg: F) -> Self::AlgTo;
}

pub struct RecursiveStruct<F> {
    // nonempty, in topological-sorted order
    elems: Vec<F>,
}

pub type RecursiveExpr2 = RecursiveStruct<Expr<usize>>;


pub trait CoRecursive<A, O> {
    fn ana<F: Fn(A) -> O>(a: A, coalg: F) -> Self;
}

impl<A, O> CoRecursive<A, O> for RecursiveStruct<Expr<usize>> {
    fn ana<F: Fn(A) -> Expr<A>>(a: A, coalg: F) -> Self {
        let mut frontier = VecDeque::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop_front() {
            let node = coalg(seed);

            let node: Expr<usize> = node.fmap_into(|aa| {
                frontier.push_back(aa);
                // this is the sketchy bit, here - idx of pointed-to element
                elems.len() + frontier.len()
            });

            elems.push(node);
        }

        Self { elems }
    }
}

impl<A> Recursive<A> for RecursiveStruct<Expr<usize>> {
    type AlgFrom = Expr<A>;

    type AlgTo = A;

    fn cata<F: FnMut(Self::AlgFrom) -> Self::AlgTo>(self, mut alg: F) -> Self::AlgTo {
        let mut results: HashMap<usize, A> = HashMap::with_capacity(self.elems.len());

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it to avoid cloning owned data
                let node = node.fmap_into(|x| results.remove(&x).expect("node not in result map"));
                alg(node)
            };
            results.insert(idx, alg_res);
        }

        // assumes nonempty recursive structure
        results.remove(&0).unwrap()
    }
}

// everything below here can be generated pretty easily given the above

pub struct RecursiveExpr {
    // nonempty, in topological-sorted order
    elems: Vec<Expr<usize>>,
}

impl RecursiveExpr {
    pub fn ana<A, F: Fn(A) -> Expr<A>>(a: A, coalg: F) -> Self {
        let mut frontier = VecDeque::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop_front() {
            let node = coalg(seed);

            let node: Expr<usize> = node.fmap_into(|aa| {
                frontier.push_back(aa);
                // this is the sketchy bit, here - idx of pointed-to element
                elems.len() + frontier.len()
            });

            elems.push(node);
        }

        Self { elems }
    }

    pub fn cata<A, F: FnMut(Expr<A>) -> A>(self, mut alg: F) -> A {
        let mut results: HashMap<usize, A> = HashMap::with_capacity(self.elems.len());

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it to avoid cloning owned data
                let node = node.fmap_into(|x| results.remove(&x).expect("node not in result map"));
                alg(node)
            };
            results.insert(idx, alg_res);
        }

        // assumes nonempty recursive structure
        results.remove(&0).unwrap()
    }

    // HAHA HOLY SHIT THIS RULES IT WORKS IT WORKS IT WORKS, GET A POSTGRES TEST GOING BECAUSE THIS RULES
    pub async fn cata_async<
        'a,
        A: Send + Sync + 'a,
        E: Send + 'a,
        F: Fn(Expr<A>) -> BoxFuture<'a, Result<A, E>> + Send + Sync + 'a,
    >(
        self,
        alg: F,
    ) -> Result<A, E> {
        let execution_graph = self.cata(|e|
            // NOTE: want to directly pass in fn but can't because borrow checker - not sure how to do this, causes spurious clippy warning
            cata_async_helper(e,  |x| alg(x)));

        execution_graph.await
    }
}

// given an async fun, build an execution graph from cata async
fn cata_async_helper<
    'a,
    A: Send + 'a,
    E: 'a,
    F: Fn(Expr<A>) -> BoxFuture<'a, Result<A, E>> + Send + Sync + 'a,
>(
    e: Expr<BoxFuture<'a, Result<A, E>>>,
    f: F,
) -> BoxFuture<'a, Result<A, E>> {
    async move {
        let e = e.try_join().await?;
        f(e).await
    }
    .boxed()
}
