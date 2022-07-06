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
use crate::recursive_naive::{arb_expr, from_ast, naive_eval};
#[cfg(test)]
use proptest::prelude::*;

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
#[cfg(test)]
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn expr_eval((expr, db_state) in arb_expr()) {
        // NOTE: this helped me find one serious bug in new cata impl, where it was doing vec pop instead of vec head_pop so switched to VecDequeue. Found minimal example, Add (0, Sub(0, 1)).
        let expr = Box::new(expr);
        let simple = naive_eval(&db_state, expr.clone());
        let complex = eval(&db_state, from_ast(expr.clone()));

        assert_eq!(simple, complex);
    }
}

// generate a bunch of expression trees and evaluate them
#[cfg(test)]
proptest! {
    #![proptest_config(ProptestConfig::with_cases(5))]
    #[test]
    fn expr_eval_pg((expr, db_state) in arb_expr()) {
        use tokio::runtime::Runtime;

        // Create the runtime
        let rt  = Runtime::new().unwrap();

        // TODO/FIMXE: mb don't bring a database up for each test lol this is trash
        let expr = Box::new(expr);
        let simple = naive_eval(&db_state, expr.clone());
        let f = crate::db::run_embedded_db("test db", |conn_str| { 
            DB::with_db(conn_str, |db| {
                let expr = from_ast(expr.clone());
                let db = db.clone();
                let db_state = db_state.clone();
                async move {
                db.init(&db_state).await.unwrap();
                eval_postgres(&db, expr).await
        }} )
        });
        let pg = rt.block_on(f).unwrap().unwrap().unwrap(); // unwrap all the different typed errors from the embed/with db/etc stuff

        assert_eq!(simple, pg);
    }
}
