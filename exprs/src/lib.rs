mod db;
pub mod linked_list;
mod recursive;
mod recursive_abstract;
pub mod recursive_naive; // only pub to enable running this in the main method

use crate::db::DBKey;
use crate::db::DB;
use crate::recursive::Expr;
use futures::future;
use futures::FutureExt;
use recursive::RecursiveExpr;
use recursive_abstract::Recursive;
use std::collections::HashMap;

#[cfg(test)]
use crate::recursive_naive::{arb_expr, from_ast, naive_eval};
#[cfg(test)]
use proptest::prelude::*;

pub fn eval(db: &HashMap<DBKey, i64>, g: RecursiveExpr) -> i64 {
    g.cata(|node| match node {
        Expr::Add(a, b) => a + b,
        Expr::Sub(a, b) => a - b,
        Expr::Mul(a, b) => a * b,
        Expr::LiteralInt(x) => x,
        Expr::DatabaseRef(x) => *db.get(&x).expect("cata eval db lookup failed"),
    })
}

pub async fn eval_async(db: &DB, g: RecursiveExpr) -> Result<i64, String> {
    let f = g.cata_async(|node| match node {
        Expr::Add(a, b) => future::ok(a + b).boxed(),
        Expr::Sub(a, b) => future::ok(a - b).boxed(),
        Expr::Mul(a, b) => future::ok(a * b).boxed(),
        Expr::LiteralInt(x) => future::ok(x).boxed(),
        Expr::DatabaseRef(key) => {
            let f = async move { db.get(&key).await.map_err(|x| x.to_string()) };
            f.boxed()
        }
    });

    f.await
}

// generate a bunch of expression trees and evaluate them
#[cfg(test)]
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn expr_eval((expr, db_state) in arb_expr()) {
        // NOTE: this helped me find one serious bug in new cata impl, where it was doing vec pop instead of vec head_pop so switched to VecDequeue. Found minimal example, Add (0, Sub(0, 1)).
        let expr = Box::new(expr);
        let simple = naive_eval(&db_state, expr.clone());
        let complex = eval(&db_state, from_ast(expr.clone()));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let async_complex = rt.block_on(eval_async(&DB::init(db_state), from_ast(expr.clone()))).unwrap();

        assert_eq!(simple, complex);
        assert_eq!(simple, async_complex);
    }
}
