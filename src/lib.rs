pub mod db;

use crate::db::DBKey;
use crate::db::DB;
use futures::future;
use futures::future::BoxFuture;
use futures::FutureExt;
use petgraph::{algo::toposort, graph::NodeIndex, Directed, Graph};
use std::collections::HashMap;
use std::future::Future;

// TODO/FIXME: test only
// Bring the macros and other important things into scope.
use proptest::prelude::*;

// or, IRL - parsed TOML or string or etc
pub fn from_ast(ast: Box<ExprAST>) -> Graph<Expr<EdgeIdx>, EdgeIdx, Directed> {
    ana(ast, |x| match *x {
        ExprAST::Add(a, b) => Expr::Add(a, b),
        ExprAST::Sub(a, b) => Expr::Sub(a, b),
        ExprAST::Mul(a, b) => Expr::Mul(a, b),
        ExprAST::LiteralInt(x) => Expr::LiteralInt(x),
        ExprAST::DatabaseRef(x) => Expr::DatabaseRef(x),
    })
}

// wow, this is surprisingly easy - can add type checking to make it really pop!
pub fn eval(db: &HashMap<DBKey, i64>, g: Graph<Expr<EdgeIdx>, EdgeIdx, Directed>) -> i64 {
    cata(g, |node| match node {
        Expr::Add(a, b) => a + b,
        Expr::Sub(a, b) => a - b,
        Expr::Mul(a, b) => a * b,
        Expr::LiteralInt(x) => x,
        Expr::DatabaseRef(x) => *db.get(&x).expect("cata eval db lookup failed"),
    })
}

type EvalError = String;

// NOTE: may be sufficiently non-idiomatic to want to avoid
// async fn async_alg<A, B>(e: Expr<Box<dyn Future<Output = A>>>, f: Fn(&Expr<A>) -> B) -> B {
//     match expr {
//         Expr::Add(a, b) => {
//             // TODO: join with early termination on Err
//             let (a,b) = future::join(a, b).await;
//             future::ok(f(a + b).boxed()
//         },
//         Expr::Sub(a, b) => future::ok(a - b).boxed(),
//         Expr::Mul(a, b) => future::ok(a * b).boxed(),
//         Expr::LiteralInt(x) => future::ok(x).boxed(),
//         Expr::DatabaseRef(key) => {
//             let f = async move { db.get(key).await.map_err(|x| x.to_string()) };
//             f.boxed()
//         }
//     }
// }

// async fn cata_async_2<
//     'a,
//     A: Send + Sync + 'a,
//     E: Send + 'a,
//     F: Fn(Expr<A>) -> BoxFuture<'a, Result<A, E>> + Send + Sync + 'a,
// >(
//     g: Graph<Expr<EdgeIdx>, EdgeIdx, Directed>,
//     alg: F,
// ) -> Result<A, E> {
//     let execution_graph = cata(g, move |e| cata_async_helper(e, alg));

//     execution_graph.await
// }

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
        match e {
            Expr::Add(a, b) => {
                // TODO: join with early termination on Err
                let (a, b) = future::try_join(a, b).await?;
                let res = f(Expr::Add(a, b)).await?;
                Ok(res)
            }
            Expr::Sub(a, b) => {
                // TODO: try_join with early termination on Err
                let (a, b) = future::try_join(a, b).await?;
                let res = f(Expr::Sub(a, b)).await?;
                Ok(res)
            }

            Expr::Mul(a, b) => {
                // TODO: try_join with early termination on Err
                let (a, b) = future::try_join(a, b).await?;
                let res = f(Expr::Mul(a, b)).await?;
                Ok(res)
            }

            Expr::LiteralInt(x) => {
                let res = f(Expr::LiteralInt(x)).await?;
                Ok(res)
            }
            Expr::DatabaseRef(key) => {
                let res = f(Expr::DatabaseRef(key)).await?;
                Ok(res)
            }
        }
    }
    .boxed()
}

// more viable, but also probably non-idiomatic - just have special-cased async
fn postgres_alg<'a>(db: &'a DB, e: Expr<BoxFuture<'a, i64>>) -> BoxFuture<'a, Result<i64, String>> {
    async move {
        match e {
            Expr::Add(a, b) => {
                // TODO: join with early termination on Err
                let (a, b) = future::join(a, b).await;
                Ok(a + b)
            }
            Expr::Sub(a, b) => {
                // TODO: join with early termination on Err
                let (a, b) = future::join(a, b).await;
                Ok(a - b)
            }

            Expr::Mul(a, b) => {
                // TODO: join with early termination on Err
                let (a, b) = future::join(a, b).await;
                Ok(a * b)
            }

            Expr::LiteralInt(x) => Ok(x),
            Expr::DatabaseRef(key) => db.get(key).await.map_err(|x| x.to_string()),
        }
    }
    .boxed()
}

// pub async fn eval_postgres_2(
//     db: &DB,
//     g: Graph<Expr<EdgeIdx>, EdgeIdx, Directed>,
// ) -> Result<i64, EvalError> {
//     let f = cata(&g, |node| {
//             let f = async move {  };
//             f.boxed()
//     }
//     );

//     f.await
// }

// forget about type checking, too many match statements. check this out instead:
pub async fn eval_postgres(
    db: &DB,
    g: Graph<Expr<EdgeIdx>, EdgeIdx, Directed>,
) -> Result<i64, EvalError> {
    let f = cata_async(&g, |node| match node {
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
        cata(from_ast(Box::new(self.clone())), |expr| match expr {
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

type EdgeIdx = usize;

#[derive(Debug, Clone, Copy)]
pub enum Expr<A> {
    Add(A, A),
    Sub(A, A),
    Mul(A, A),
    LiteralInt(i64),
    DatabaseRef(DBKey),
}

fn expr_leaves<A>(e: &Expr<A>) -> impl Iterator<Item = &A> {
    let slice = match e {
        Expr::Add(a, b) => vec![a, b],
        Expr::Sub(a, b) => vec![a, b],
        Expr::Mul(a, b) => vec![a, b],
        Expr::LiteralInt(_) | Expr::DatabaseRef(_) => Vec::with_capacity(0),
    };

    slice.into_iter()
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

// PLAN: implement this then run some tests
fn ana<A, F: Fn(A) -> Expr<A>>(a: A, coalg: F) -> Graph<Expr<EdgeIdx>, EdgeIdx, Directed> {
    let mut frontier: Vec<(usize, A)> = Vec::new();
    frontier.push((0, a));

    // we don't have graph indices yet so we create an internal index via monotonic increase of usize
    // start with '1' because the frontier already has 1 value
    let mut next_idx: usize = 1;

    // collect nodes to create, neccessary because each node are created before their children are expanded from seed values
    let mut nodes_to_create: Vec<(usize, Expr<usize>)> = Vec::new();

    while let Some((node_idx, seed)) = frontier.pop() {
        let node = coalg(seed);

        let node: Expr<usize> = fmap_into(node, |aa| {
            let idx = next_idx;
            next_idx += 1;
            frontier.push((idx, aa));
            idx
        });

        nodes_to_create.push((node_idx, node));
    }

    // assume topo ordering, start with leaf nodes (at end) and insert backwards.
    // this works b/c definitionally, nodes precede their children because they
    // must have existed in nodes before adding child seeds to frontier
    // NOTE: this might fail if we end up in a DAG (instead of tree) state? need to think about this <- FIXME?
    // NOTE: nvm lol this fn can't generate DAGs, might have duplicates b/c no Eq constraint on 'a', will
    //       just generate duplicate branches w/ no structural sharing, it's fine. that's not even required.
    let mut idx_to_graph_idx = HashMap::new();

    let mut graph = Graph::new();

    // boilerplate - build graph from nodes_to_create
    for (idx, expr) in nodes_to_create.into_iter().rev() {
        // collect edges to add before adding node
        let mut edges_to_add: Vec<(NodeIndex, usize)> = Vec::new();
        for edge in expr_leaves(&expr) {
            // can just remove mappings b/c each is only used once (no structural sharing yet)
            let to_graph_idx = idx_to_graph_idx
                .remove(edge)
                .ok_or("broken link during 'ana'")
                .unwrap();

            edges_to_add.push((to_graph_idx, *edge))
        }

        let graph_idx = graph.add_node(expr);
        for (to, weight) in edges_to_add.into_iter() {
            graph.add_edge(graph_idx, to, weight);
        }

        idx_to_graph_idx.insert(idx, graph_idx);
    }

    graph
}

// NOTE: assumes that there is one node that is the 'head' node, which will always be at the beginning of a topo sort
// NOTE: can work with graph subsets later, with specified head nodes and etc
// NOTE NOTE NOTE: does not work with grpahs, b/c that makes ownership cleaner
fn cata<A, F: FnMut(Expr<A>) -> A>(g: Graph<Expr<EdgeIdx>, EdgeIdx, Directed>, mut alg: F) -> A {
    let topo_order = toposort(&g, None).expect("graph should not have cycles");

    let head_node = topo_order[0]; // throws error if empty graph, TODO/FIXME (maybe?)

    let mut results: HashMap<NodeIndex, A> = HashMap::with_capacity(topo_order.len());

    for idx in topo_order.into_iter().rev() {
        let alg_res = {
            let node = g.node_weight(idx).unwrap();
            let edges = g.edges(idx);
            // NOTE I think that the second node in the edgeref is ALWAYS what I want,
            // if not or if nondeterministic order can compare both to current idx and take one that !=
            let mut edge_map: HashMap<EdgeIdx, NodeIndex> =
                edges.map(|e| (*e.weight(), e.node[1])).collect();

            let node =
                traverse_into(*node, |x| edge_map.remove(&x).ok_or("ref not in edge map")).unwrap();
            // cannot remove result from map (to acquire ownership) because doing so limits us to trees and would cause failures on DAGs
            let node = traverse_into(node, |x| results.remove(&x).ok_or("node not in result map"))
                .unwrap();
            alg(node)
        };
        results.insert(idx, alg_res);
    }

    results.remove(&head_node).unwrap()
}

async fn cata_async<
    'a,
    A: 'a,
    E,
    Fut: Future<Output = Result<A, E>> + 'a,
    F: Fn(Expr<&'_ A>) -> Fut,
>(
    g: &'a Graph<Expr<EdgeIdx>, EdgeIdx, Directed>,
    alg: F,
) -> Result<A, E> {
    let topo_order = toposort(&g, None).unwrap();

    let head_node = topo_order[0]; // throws error if empty graph, TODO/FIXME

    let mut results: HashMap<NodeIndex, A> = HashMap::with_capacity(topo_order.len());

    for idx in topo_order.into_iter().rev() {
        let alg_res = {
            let node = g.node_weight(idx).unwrap();
            let edges = g.edges(idx);
            // NOTE I think that the second node in the edgeref is ALWAYS what I want,
            // if not or if nondeterministic order can compare both to current idx and take one that !=
            let mut edge_map: HashMap<EdgeIdx, NodeIndex> =
                edges.map(|e| (*e.weight(), e.node[1])).collect();

            let node =
                traverse_into(*node, |x| edge_map.remove(&x).ok_or("ref not in edge map")).unwrap();
            // cannot remove result from map (to acquire ownership) because doing so limits us to trees and would cause failures on DAGs
            let node =
                traverse_into(node, |x| results.get(&x).ok_or("node not in result map")).unwrap();
            alg(node).await?
        };
        results.insert(idx, alg_res);
    }

    Ok(results.remove(&head_node).unwrap())
}

// how would I express an expr graph in petgraph?
// idea: expressions along the edges and evaluated values in the nodes?
//       problem - can't have A + B I think?
// idea: full expr in the node and positional reference to edges - edge 1, 2, 3, etc. a bit janky, but works fine I guess
// yes: the expr is the node, each sub-expr is an outgoing edge. can't rely on edge ordering, so just throw some usize's at it
//      there won't be more

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
