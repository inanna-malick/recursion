mod db;
mod recursive;
pub mod recursive_naive; // only pub to enable running this in the main method

use crate::db::DBKey;
use crate::db::DB;
use crate::recursive::Expr;
use futures::future;
use futures::FutureExt;
use recursive::RecursiveExpr;
use std::collections::HashMap;

#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use crate::recursive_naive::{from_ast, naive_eval, arb_expr};

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
