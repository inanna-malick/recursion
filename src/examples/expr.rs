pub mod db;
pub mod eval;
pub mod naive;

use crate::examples::expr::db::DBKey;
use crate::recursive::{Functor, Recursive, RecursiveStruct};
use futures::future;
use futures::future::BoxFuture;
use futures::FutureExt;

/// Simple expression language with some operations on integers
#[derive(Debug, Clone, Copy)]
pub enum Expr<A> {
    Add(A, A),
    Sub(A, A),
    Mul(A, A),
    LiteralInt(i64),
    DatabaseRef(DBKey),
}

impl<A, B> Functor<B> for Expr<A> {
    type To = Expr<B>;
    type Unwrapped = A;

    fn fmap<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            Expr::Add(a, b) => Expr::Add(f(a), f(b)),
            Expr::Sub(a, b) => Expr::Sub(f(a), f(b)),
            Expr::Mul(a, b) => Expr::Mul(f(a), f(b)),
            Expr::LiteralInt(x) => Expr::LiteralInt(x),
            Expr::DatabaseRef(x) => Expr::DatabaseRef(x),
        }
    }
}

// this is, like, basically fine?
impl<'a, B: 'a> Functor<B> for &'a Expr<usize> {
    type To = Expr<B>;
    type Unwrapped = usize;

    fn fmap<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            Expr::Add(a, b) => Expr::Add(f(*a), f(*b)),
            Expr::Sub(a, b) => Expr::Sub(f(*a), f(*b)),
            Expr::Mul(a, b) => Expr::Mul(f(*a), f(*b)),
            Expr::LiteralInt(x) => Expr::LiteralInt(*x),
            Expr::DatabaseRef(x) => Expr::DatabaseRef(*x),
        }
    }
}

pub type RecursiveExpr = RecursiveStruct<Expr<usize>>;

impl RecursiveExpr {
    /// build a parallel execution graph from a recursive expr and an algebra
    pub async fn cata_async<
        'a,
        A: Send + Sync + 'a,
        E: Send + 'a,
        F: Fn(Expr<A>) -> BoxFuture<'a, Result<A, E>> + Send + Sync + 'a,
    >(
        self,
        alg: &'a F,
    ) -> Result<A, E> {
        let execution_graph = self.fold(|e| cata_async_helper(e, alg));

        execution_graph.await
    }
}

fn cata_async_helper<
    'a,
    A: Send + 'a,
    E: 'a,
    F: Fn(Expr<A>) -> BoxFuture<'a, Result<A, E>> + Send + Sync + 'a,
>(
    e: Expr<BoxFuture<'a, Result<A, E>>>,
    f: &'a F,
) -> BoxFuture<'a, Result<A, E>> {
    async move {
        let e = e.try_join().await?;
        f(e).await
    }
    .boxed()
}

impl<A, E> Expr<BoxFuture<'_, Result<A, E>>> {
    async fn try_join(self) -> Result<Expr<A>, E> {
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
