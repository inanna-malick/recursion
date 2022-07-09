use crate::examples::expr::db::DBKey;
use crate::examples::expr::*;
use crate::recursive::CoRecursive;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use std::collections::HashMap;

/// simple naive representation of a recursive expression AST.
#[derive(Debug, Clone)]
pub enum ExprAST {
    Add(Box<ExprAST>, Box<ExprAST>),
    Sub(Box<ExprAST>, Box<ExprAST>),
    Mul(Box<ExprAST>, Box<ExprAST>),
    LiteralInt(i64),
    DatabaseRef(DBKey),
}

// or, IRL - parsed TOML or string or etc
pub fn from_ast(ast: Box<ExprAST>) -> RecursiveExpr {
    RecursiveExpr::unfold(ast, |x| match *x {
        ExprAST::Add(a, b) => Expr::Add(a, b),
        ExprAST::Sub(a, b) => Expr::Sub(a, b),
        ExprAST::Mul(a, b) => Expr::Mul(a, b),
        ExprAST::LiteralInt(x) => Expr::LiteralInt(x),
        ExprAST::DatabaseRef(x) => Expr::DatabaseRef(x),
    })
}

impl ExprAST {
    #[cfg(test)]
    fn keys(&self) -> Vec<DBKey> {
        let mut keys = Vec::new();
        // TODO: totally unneeded clone here, fixme
        from_ast(Box::new(self.clone())).fold(|expr| match expr {
            Expr::DatabaseRef(k) => keys.push(k),
            _ => {}
        });

        keys
    }
}

#[cfg(test)]
pub fn arb_expr() -> impl Strategy<Value = (ExprAST, HashMap<DBKey, i64>)> {
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
