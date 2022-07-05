pub mod db;
pub mod recursive;
pub mod recursive_naive;

use crate::db::DBKey;
use crate::db::DB;
use crate::recursive_naive::*;
use futures::future;
use futures::future::BoxFuture;
use futures::FutureExt;
use std::collections::HashMap;
use std::collections::VecDeque;

#[cfg(test)]
use proptest::prelude::*;


// or, IRL - parsed TOML or string or etc
pub fn from_ast(ast: Box<ExprAST>) -> RecursiveExpr {
    RecursiveExpr::ana(ast, |x| match *x {
        ExprAST::Add(a, b) => Expr::Add(a, b),
        ExprAST::Sub(a, b) => Expr::Sub(a, b),
        ExprAST::Mul(a, b) => Expr::Mul(a, b),
        ExprAST::LiteralInt(x) => Expr::LiteralInt(x),
        ExprAST::DatabaseRef(x) => Expr::DatabaseRef(x),
    })
}

// wow, this is surprisingly easy - can add type checking to make it really pop!
pub fn eval(db: &HashMap<DBKey, i64>, g: RecursiveExpr) -> i64 {
    g.cata(|node| {
        println!("eval: {:?}", node);
        match node {
            Expr::Add(a, b) => a + b,
            Expr::Sub(a, b) => a - b,
            Expr::Mul(a, b) => a * b,
            Expr::LiteralInt(x) => x,
            Expr::DatabaseRef(x) => *db.get(&x).expect("cata eval db lookup failed"),
        }
    })
}

// forget about type checking, too many match statements. check this out instead:
pub async fn eval_postgres(db: &DB, g: RecursiveExpr) -> Result<i64, EvalError> {
    let f = g.cata_async(|node| match node {
        Expr::Add(a, b) => future::ok(a + b).boxed(),
        Expr::Sub(a, b) => future::ok(a - b).boxed(),
        Expr::Mul(a, b) => future::ok(a * b).boxed(),
        Expr::LiteralInt(x) => future::ok(x).boxed(),
        Expr::DatabaseRef(key) => {
            let f = async move { db.get(key).await.map_err(|x| x.to_string()) };
            f.boxed()
        }
    });

    f.await
}

type EvalError = String;

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

#[derive(Debug, Clone, Copy)]
pub enum Expr<A> {
    Add(A, A),
    Sub(A, A),
    Mul(A, A),
    LiteralInt(i64),
    DatabaseRef(DBKey),
}

fn fmap_into<A, B, F: FnMut(A) -> B>(e: Expr<A>, mut f: F) -> Expr<B> {
    match e {
        Expr::Add(a, b) => Expr::Add(f(a), f(b)),
        Expr::Sub(a, b) => Expr::Sub(f(a), f(b)),
        Expr::Mul(a, b) => Expr::Mul(f(a), f(b)),
        Expr::LiteralInt(x) => Expr::LiteralInt(x),
        Expr::DatabaseRef(x) => Expr::DatabaseRef(x),
    }
}

pub struct RecursiveExpr {
    // nonempty, in topological-sorted order
    elems: Vec<Expr<usize>>,
}

impl RecursiveExpr {
    fn ana<A, F: Fn(A) -> Expr<A>>(a: A, coalg: F) -> Self {
        let mut frontier = VecDeque::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop_front() {
            let node = coalg(seed);

            let node: Expr<usize> = fmap_into(node, |aa| {
                frontier.push_back(aa);
                // this is the sketchy bit, here
                let idx = elems.len() + frontier.len();
                idx
            });

            elems.push(node);
        }

        Self { elems }
    }

    fn cata<A, F: FnMut(Expr<A>) -> A>(self, mut alg: F) -> A {
        let mut results: HashMap<usize, A> = HashMap::with_capacity(self.elems.len());

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it to avoid cloning owned data
                let node = fmap_into(node, |x| {
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
    async fn cata_async<
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

// generate a bunch of expression trees and evaluate them
// NOTE: this helped me find one serious bug in new cata impl, where it was doing vec pop instead of vec head_pop so switched to VecDequeue. Found minimal example, Add (0, Sub(0, 1)). 
#[cfg(test)]
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn evals_correctly((expr, db_state) in arb_expr()) {
        let expr = Box::new(expr);
        let simple = naive_eval(&db_state, expr.clone());
        let complex = eval(&db_state, from_ast(expr.clone()));

        assert_eq!(simple, complex);
    }

    // #![proptest_config(ProptestConfig::with_cases(500))]
    // #[test]
    // fn evals_correctly_postgres(expr in arb_expr()) {
    //     // TODO/FIMXE: mb don't bring a database up for each test lol
    //     let expr = Box::new(expr);
    //     let db_state = HashMap::new();
    //     let complex = eval(&db_state, from_ast(expr.clone()));

    //     assert_eq!(simple, complex);
    // }
}
