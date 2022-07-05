use crate::db::DBKey;
use futures::future;
use futures::future::BoxFuture;
use futures::FutureExt;
use std::collections::HashMap;
use std::collections::VecDeque;


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

impl<A, E> Expr<BoxFuture<'_, Result<A, E>>> {
    async fn try_join_expr(self) -> Result<Expr<A>, E> {
        match self {
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
}
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
                // this is the sketchy bit, here
                let idx = elems.len() + frontier.len();
                idx
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
                let node = node.fmap_into( |x| {
                    results.remove(&x).expect("node not in result map")
                });
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
        let execution_graph = self.cata(|e| cata_async_helper(e, |x| alg(x)));

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
        let e = e.try_join_expr().await?;
        f(e).await
    }
    .boxed()
}
