use crate::examples::expr::db::DBKey;
use crate::examples::expr::db::DB;
use crate::examples::expr::{Expr, RecursiveExpr};
use crate::recursive::Recursive;
use futures::future;
use futures::FutureExt;
use std::collections::HashMap;

#[cfg(test)]
use crate::examples::expr::naive::arb_expr;
use crate::examples::expr::naive::{from_ast, ExprAST};
#[cfg(test)]
use proptest::prelude::*;

pub fn eval(db: &HashMap<DBKey, i64>, g: &RecursiveExpr) -> i64 {
    g.as_ref().fold(|node: Expr<i64>| match node {
        Expr::Add(a, b) => a + b,
        Expr::Sub(a, b) => a - b,
        Expr::Mul(a, b) => a * b,
        Expr::LiteralInt(x) => x,
        Expr::DatabaseRef(x) => *db.get(&x).expect("cata eval db lookup failed"),
    })
}

pub async fn eval_async(db: &DB, g: RecursiveExpr) -> Result<i64, String> {
    let alg = |node| match node {
        Expr::Add(a, b) => future::ok(a + b).boxed(),
        Expr::Sub(a, b) => future::ok(a - b).boxed(),
        Expr::Mul(a, b) => future::ok(a * b).boxed(),
        Expr::LiteralInt(x) => future::ok(x).boxed(),
        Expr::DatabaseRef(key) => {
            let f = async move { db.get(&key).await.map_err(|x| x) };
            f.boxed()
        }
    };
    let f = g.cata_async(&alg);

    f.await
}

pub fn naive_eval(db: &HashMap<DBKey, i64>, expr: &ExprAST) -> i64 {
    match expr {
        ExprAST::Add(a, b) => naive_eval(db, a) + naive_eval(db, b),
        ExprAST::Sub(a, b) => naive_eval(db, a) - naive_eval(db, b),
        ExprAST::Mul(a, b) => naive_eval(db, a) * naive_eval(db, b),
        ExprAST::DatabaseRef(x) => *db.get(&x).expect("naive eval db lookup failed"),
        ExprAST::LiteralInt(x) => *x,
    }
}

// generate a bunch of expression trees and evaluate them
#[cfg(test)]
proptest! {
    #[test]
    fn expr_eval((expr, db_state) in arb_expr()) {
        // NOTE: this helped me find one serious bug in new cata impl, where it was doing vec pop instead of vec head_pop so switched to VecDequeue. Found minimal example, Add (0, Sub(0, 1)).
        let expr = Box::new(expr);
        let simple = naive_eval(&db_state, &expr);
        let complex = eval(&db_state, &from_ast(expr.clone()));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let async_complex = rt.block_on(eval_async(&DB::init(db_state), from_ast(expr))).unwrap();

        assert_eq!(simple, complex);
        assert_eq!(simple, async_complex);
    }
}
