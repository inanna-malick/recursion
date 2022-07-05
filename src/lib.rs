pub mod db;

use crate::db::DBKey;
use crate::db::DB;
use futures::future;
use futures::future::BoxFuture;
use futures::FutureExt;
use std::collections::HashMap;

// TODO/FIXME: test only
// Bring the macros and other important things into scope.
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
    g.cata(|node| match node {
        Expr::Add(a, b) => a + b,
        Expr::Sub(a, b) => a - b,
        Expr::Mul(a, b) => a * b,
        Expr::LiteralInt(x) => x,
        Expr::DatabaseRef(x) => *db.get(&x).expect("cata eval db lookup failed"),
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

// NOTE: using this instead of a parsed JSON AST or some other similar serialized repr for conciseness
// NOTE: fully recursive data structure of unknown size lol
#[derive(Debug, Clone)]
pub enum ExprAST {
    Add(Box<ExprAST>, Box<ExprAST>),
    Sub(Box<ExprAST>, Box<ExprAST>),
    Mul(Box<ExprAST>, Box<ExprAST>),
    LiteralInt(i64),
    DatabaseRef(DBKey),
}

impl ExprAST {
    #[cfg(test)]
    fn keys(&self) -> Vec<DBKey> {
        let mut keys = Vec::new();
        // TODO: totally unneeded clone here, fixme
        from_ast(Box::new(self.clone())).cata(|expr| match expr {
            Expr::DatabaseRef(k) => keys.push(k),
            _ => {}
        });

        keys
    }
}

#[cfg(test)]
fn naive_eval(db: &HashMap<DBKey, i64>, expr: Box<ExprAST>) -> i64 {
    match *expr {
        ExprAST::Add(a, b) => naive_eval(db, a) + naive_eval(db, b),
        ExprAST::Sub(a, b) => naive_eval(db, a) - naive_eval(db, b),
        ExprAST::Mul(a, b) => naive_eval(db, a) * naive_eval(db, b),
        ExprAST::DatabaseRef(x) => *db.get(&x).expect("naive eval db lookup failed"),
        ExprAST::LiteralInt(x) => x,
    }
}

#[cfg(test)]
fn arb_expr() -> impl Strategy<Value = (ExprAST, HashMap<DBKey, i64>)> {
    // TODO: figure out getting db values out, idk flatmap map to generate tuple of expr and DB state I guess
    let leaf = prop_oneof![
        any::<i8>().prop_map(|x| ExprAST::LiteralInt(x as i64)),
        any::<u32>().prop_map(|u| ExprAST::DatabaseRef(DBKey(u)))
    ];
    let expr = leaf.prop_recursive(
        8,   // 8 levels deep
        256, // Shoot for maximum size of 256 nodes
        10,  // We put up to 10 items per collection
        |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| ExprAST::Add(Box::new(a), Box::new(b))),
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| ExprAST::Sub(Box::new(a), Box::new(b))),
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| ExprAST::Mul(Box::new(a), Box::new(b))),
            ]
        },
    );

    // TODO: generate a bunch of i64's and assign them to keys here, 1 per key
    // TODO: generate vec of fixed size - size of keys list, then map over it and zip with keys to get db state
    expr.prop_flat_map(|e| {
        let db = e
            .keys()
            .into_iter()
            .map(|k| any::<i8>().prop_map(move |v| (k, v as i64)))
            .collect::<Vec<_>>()
            .prop_map(|kvs| kvs.into_iter().collect::<HashMap<DBKey, i64>>());

        // fixme remove clone
        db.prop_map(move |db| (e.clone(), db))
    })
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

fn traverse_into<A, B, E, F: FnMut(A) -> Result<B, E>>(e: Expr<A>, mut f: F) -> Result<Expr<B>, E> {
    Ok(match e {
        Expr::Add(a, b) => Expr::Add(f(a)?, f(b)?),
        Expr::Sub(a, b) => Expr::Sub(f(a)?, f(b)?),
        Expr::Mul(a, b) => Expr::Mul(f(a)?, f(b)?),
        Expr::LiteralInt(x) => Expr::LiteralInt(x),
        Expr::DatabaseRef(x) => Expr::DatabaseRef(x),
    })
}

pub struct RecursiveExpr {
    elems: HashMap<usize, Expr<usize>>,
    topo_order: Vec<usize>, // guaranteed at least one element by construction
}

impl RecursiveExpr {
    fn cata<A, F: FnMut(Expr<A>) -> A>(mut self, mut alg: F) -> A {
        let head_node = self.topo_order[0]; // throws error if empty graph, TODO/FIXME (maybe?)

        let mut results: HashMap<usize, A> = HashMap::with_capacity(self.elems.len());

        for idx in self.topo_order.into_iter().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it to avoid cloning owned data
                let node = self.elems.remove(&idx).unwrap();
                let node =
                    traverse_into(node, |x| results.remove(&x).ok_or("node not in result map"))
                        .unwrap();
                alg(node)
            };
            results.insert(idx, alg_res);
        }

        results.remove(&head_node).unwrap()
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

    fn ana<A, F: Fn(A) -> Expr<A>>(a: A, coalg: F) -> Self {
        let mut frontier: Vec<(usize, A)> = vec![(0, a)];
        let mut topo_order = vec![0];

        // we don't have graph indices yet so we create an internal index via monotonic increase of usize
        // start with '1' because the frontier already has 1 value
        let mut next_idx: usize = 1;

        // collect nodes to create, neccessary because each node are created before their children are expanded from seed values
        let mut elems = HashMap::new();

        while let Some((node_idx, seed)) = frontier.pop() {
            let node = coalg(seed);

            let node: Expr<usize> = fmap_into(node, |aa| {
                let idx = next_idx;
                next_idx += 1;
                frontier.push((idx, aa));
                topo_order.push(idx);
                idx
            });

            elems.insert(node_idx, node);
        }

        // assume topo ordering, start with leaf nodes (at end) and insert backwards.
        // this works b/c definitionally, nodes precede their children because they
        // must have existed in nodes before adding child seeds to frontier
        Self { elems, topo_order }
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
