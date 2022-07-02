
use std::{collections::HashMap, hash::Hash};

use petgraph::{algo::toposort, graph::NodeIndex, Directed, Graph};

// TODO/FIXME: test only
// Bring the macros and other important things into scope.
use proptest::prelude::*;

// or, IRL - parsed TOML or string or etc
pub fn from_ast(ast: Box<ExprAST>) -> Graph<ExprRaw, EdgeIdx, Directed> {
    ana(ast, |x| match *x {
        ExprAST::Add(a, b) => Expr::Add(a, b),
        ExprAST::Sub(a, b) => Expr::Sub(a, b),
        ExprAST::Mul(a, b) => Expr::Mul(a, b),
        ExprAST::LiteralInt(x) => Expr::LiteralInt(x),
    })
}

// wow, this is surprisingly easy - can add type checking to make it really pop!
pub fn eval(g: Graph<ExprRaw, EdgeIdx, Directed>) -> i64 {
    cata(g, |node| match node {
        Expr::Add(a, b) => a + b,
        Expr::Sub(a, b) => a - b,
        Expr::Mul(a, b) => a * b,
        Expr::LiteralInt(x) => x,
    })
}

// NOTE: using this instead of a parsed JSON AST or some other similar serialized repr for conciseness
// NOTE: fully recursive data structure of unknown size lol

#[derive(Debug, Clone)]
pub enum ExprAST {
    Add(Box<ExprAST>, Box<ExprAST>),
    Sub(Box<ExprAST>, Box<ExprAST>),
    Mul(Box<ExprAST>, Box<ExprAST>),
    LiteralInt(i64),
}

fn naive_eval(expr: Box<ExprAST>) -> i64 {
    match *expr {
        ExprAST::Add(a, b) => naive_eval(a) + naive_eval(b),
        ExprAST::Sub(a, b) => naive_eval(a) - naive_eval(b),
        ExprAST::Mul(a, b) => naive_eval(a) * naive_eval(b),
        ExprAST::LiteralInt(x) => x,
    }
}


fn arb_expr() -> impl Strategy<Value = ExprAST> {
    let leaf = prop_oneof![
        any::<i8>().prop_map(|x| ExprAST::LiteralInt(x as i64)),
    ];
    leaf.prop_recursive(
      8, // 8 levels deep
      256, // Shoot for maximum size of 256 nodes
      10, // We put up to 10 items per collection
      |inner| prop_oneof![
          (inner.clone(), inner.clone()).prop_map(|(a,b)| ExprAST::Add(Box::new(a),Box::new(b))),
          (inner.clone(), inner.clone()).prop_map(|(a,b)| ExprAST::Sub(Box::new(a),Box::new(b))),
          (inner.clone(), inner.clone()).prop_map(|(a,b)| ExprAST::Mul(Box::new(a),Box::new(b))),
      ])
}


type EdgeIdx = usize;

// note: all expr evaluate to int, will add bool later
pub enum ExprRaw {
    Add(EdgeIdx, EdgeIdx),
    Sub(EdgeIdx, EdgeIdx),
    Mul(EdgeIdx, EdgeIdx),
    LiteralInt(i64),
}

// TODO: borrowed instead of cloned, mb
enum Expr<A> {
    Add(A, A),
    Sub(A, A),
    Mul(A, A),
    LiteralInt(i64),
}

fn coconvert(e: &Expr<EdgeIdx>) -> ExprRaw {
    match e {
        Expr::Add(a, b) => ExprRaw::Add(*a, *b),
        Expr::Sub(a, b) => ExprRaw::Sub(*a, *b),
        Expr::Mul(a, b) => ExprRaw::Mul(*a, *b),
        Expr::LiteralInt(x) => ExprRaw::LiteralInt(*x),
    }
}

fn convert(raw: &ExprRaw) -> Expr<EdgeIdx> {
    match raw {
        ExprRaw::Add(a, b) => Expr::Add(*a, *b),
        ExprRaw::Sub(a, b) => Expr::Sub(*a, *b),
        ExprRaw::Mul(a, b) => Expr::Mul(*a, *b),
        ExprRaw::LiteralInt(x) => Expr::LiteralInt(*x),
    }
}

fn fmap_into<A, B, F: FnMut(A) -> B>(e: Expr<A>, mut f: F) -> Expr<B> {
    match e {
        Expr::Add(a, b) => Expr::Add(f(a), f(b)),
        Expr::Sub(a, b) => Expr::Sub(f(a), f(b)),
        Expr::Mul(a, b) => Expr::Mul(f(a), f(b)),
        Expr::LiteralInt(x) => Expr::LiteralInt(x),
    }
}

fn traverse<A, B, E, F: FnMut(&A) -> Result<B, E>>(e: &Expr<A>, mut f: F) -> Result<Expr<B>, E> {
    Ok(match e {
        Expr::Add(a, b) => Expr::Add(f(a)?, f(b)?),
        Expr::Sub(a, b) => Expr::Sub(f(a)?, f(b)?),
        Expr::Mul(a, b) => Expr::Mul(f(a)?, f(b)?),
        Expr::LiteralInt(x) => Expr::LiteralInt(*x),
    })
}

// PLAN: implement this then run some tests
fn ana<A, F: Fn(A) -> Expr<A>>(a: A, coalg: F) -> Graph<ExprRaw, EdgeIdx, Directed> {
    let mut frontier: Vec<(usize, A)> = Vec::new();
    frontier.push((0, a));

    let mut next_idx: usize = 1;

    let mut nodes: Vec<(usize, Expr<usize>)> = Vec::new();

    while let Some((node_idx, seed)) = frontier.pop() {
        let node = coalg(seed);

        let node: Expr<usize> = fmap_into(node, |aa| {
            let idx = next_idx;
            next_idx += 1;
            frontier.push((idx, aa));
            idx
        });

        nodes.push((node_idx, node));
    }

    let mut graph = Graph::new();

    // assume topo ordering, start with leaf nodes (at end) and insert backwards.
    // this works b/c definitionally, nodes precede their children because they
    // must have existed in nodes before adding child seeds to frontier
    // NOTE: this might fail if we end up in a DAG (instead of tree) state? need to think about this <- FIXME?
    // NOTE: nvm lol this fn can't generate DAGs, might have duplicates b/c no Eq constraint on 'a', will
    //       just generate duplicate branches w/ no structural sharing, it's fine. that's not even required.
    let mut idx_to_graph_idx = HashMap::new();

    for (idx, expr) in nodes.into_iter().rev() {
        let graph_idx = graph.add_node(coconvert(&expr));
        idx_to_graph_idx.insert(idx, graph_idx);

        // add edges
        fmap_into(expr, |x| {
            // can just remove mappings b/c each is only used once (no structural sharing yet)
            let to_graph_idx = idx_to_graph_idx
                .remove(&x)
                .ok_or("broken link during 'ana'")
                .unwrap();

            graph.add_edge(graph_idx, to_graph_idx, x);
        });
    }

    graph
}

// NOTE: assumes that there is one node that is the 'head' node, which will always be at the beginning of a topo sort
// NOTE: can work with graph subsets later, with specified head nodes and etc
fn cata<A, F: Fn(Expr<&A>) -> A>(g: Graph<ExprRaw, EdgeIdx, Directed>, alg: F) -> A {
    let topo_order = toposort(&g, None).unwrap();

    let head_node = topo_order[0]; // throws error if empty graph, TODO/FIXME

    let mut results: HashMap<NodeIndex, A> = HashMap::with_capacity(topo_order.len());

    for idx in topo_order.into_iter().rev() {
        let alg_res = {
            let node = convert(g.node_weight(idx).unwrap());
            // NOTE: may want other direction, depends on topo sort results, currently just defaults to outgoing edges
            let edges = g.edges(idx);
            // NOTE I think that the second node in the edgeref is what I want,
            // if not or if nondeterministic order can compare both to current idx and take one that !=
            let edge_map: HashMap<EdgeIdx, NodeIndex> =
                edges.map(|e| (*e.weight(), e.node[1])).collect();

            let node = traverse(&node, |x| edge_map.get(x).ok_or("ref not in edge map")).unwrap();
            let node = traverse(&node, |x| results.get(x).ok_or("node not in result map")).unwrap();
            alg(node)
        };
        results.insert(idx, alg_res);
    }

    results.remove(&head_node).unwrap()
}

// how would I express an expr graph in petgraph?
// idea: expressions along the edges and evaluated values in the nodes?
//       problem - can't have A + B I think?
// idea: full expr in the node and positional reference to edges - edge 1, 2, 3, etc. a bit janky, but works fine I guess
// yes: the expr is the node, each sub-expr is an outgoing edge. can't rely on edge ordering, so just throw some usize's at it
//      there won't be more







proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn evals_correctly(expr in arb_expr()) {
        let expr = Box::new(expr);
        let simple = naive_eval(expr.clone());
        let complex = eval(from_ast(expr.clone()));

        assert_eq!(simple, complex);
    }
}