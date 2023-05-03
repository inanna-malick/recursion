use crate::expr::*;
use proptest::prelude::*;
use recursion::map_layer::Project;
use recursion_schemes::functor::Functor;
use recursion_schemes::recursive::Recursive;

/// simple naive representation of a recursive expression AST.
#[derive(Debug, Clone)]
pub enum ExprAST {
    Add(Box<ExprAST>, Box<ExprAST>),
    Sub(Box<ExprAST>, Box<ExprAST>),
    Mul(Box<ExprAST>, Box<ExprAST>),
    LiteralInt(i64),
}

impl Recursive for &ExprAST {
    type FunctorToken = Expr<PartiallyApplied>;

    fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
        match self {
            ExprAST::Add(a, b) => Expr::Add(a, b),
            ExprAST::Sub(a, b) => Expr::Sub(a, b),
            ExprAST::Mul(a, b) => Expr::Mul(a, b),
            ExprAST::LiteralInt(x) => Expr::LiteralInt(*x),
        }
    }
}

// impl RecursiveAsync for Box<ExprAST> {
//     type JoinFutureToken = Expr<PartiallyApplied>;

//     fn into_layer(
//         self,
//     ) -> BoxFuture<
//         'static,
//         <<<Self as RecursiveAsync>::JoinFutureToken as JoinFuture>::FunctorToken as Functor>::Layer<
//             Self,
//         >,
//     > {
//         async {
//             match *self {
//                 ExprAST::Add(a, b) => Expr::Add(a, b),
//                 ExprAST::Sub(a, b) => Expr::Sub(a, b),
//                 ExprAST::Mul(a, b) => Expr::Mul(a, b),
//                 ExprAST::LiteralInt(x) => Expr::LiteralInt(x),
//             }
//         }
//         .boxed()
//     }
// }

pub fn generate_layer(x: &ExprAST) -> Expr<&ExprAST> {
    match x {
        ExprAST::Add(a, b) => Expr::Add(a, b),
        ExprAST::Sub(a, b) => Expr::Sub(a, b),
        ExprAST::Mul(a, b) => Expr::Mul(a, b),
        ExprAST::LiteralInt(x) => Expr::LiteralInt(*x),
    }
}

pub fn generate_layer_2(x: ExprAST) -> Expr<ExprAST> {
    match x {
        ExprAST::Add(a, b) => Expr::Add(*a, *b),
        ExprAST::Sub(a, b) => Expr::Sub(*a, *b),
        ExprAST::Mul(a, b) => Expr::Mul(*a, *b),
        ExprAST::LiteralInt(x) => Expr::LiteralInt(x),
    }
}

impl Project for &ExprAST {
    type To = Expr<Self>;

    fn project(self) -> Self::To {
        generate_layer(self)
    }
}

pub fn arb_expr() -> impl Strategy<Value = ExprAST> {
    let leaf = prop_oneof![any::<i8>().prop_map(|x| ExprAST::LiteralInt(x as i64)),];
    leaf.prop_recursive(
        8,   // 8 levels deep
        256, // Shoot for maximum size of 256 nodes
        10,  // We put up to 10 items per collection
        |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| ExprAST::Add(Box::new(a), Box::new(b))),
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| ExprAST::Sub(Box::new(a), Box::new(b))),
                (inner.clone(), inner).prop_map(|(a, b)| ExprAST::Mul(Box::new(a), Box::new(b))),
            ]
        },
    )
}
