use crate::db::DBKey;
use futures::future;
use futures::future::BoxFuture;
use futures::FutureExt;
use std::collections::HashMap;
use std::collections::VecDeque;

pub trait Functor<B> {
    type Unwrapped;
    type To;
    fn fmap_into<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To;
}

pub struct RecursiveStruct<F> {
    // nonempty, in topological-sorted order
    elems: Vec<F>,
}

pub trait CoRecursive<A, O> {
    fn ana<F: Fn(A) -> O>(a: A, coalg: F) -> Self;
}

// yo what the fuck how how how does this compile hahah is this fully generic recursion schemes in rust? lmao
impl<A, U, O: Functor<usize, Unwrapped = A, To = U>> CoRecursive<A, O> for RecursiveStruct<U> {
    fn ana<F: Fn(A) -> O>(a: A, coalg: F) -> Self {
        let mut frontier = VecDeque::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop_front() {
            let node = coalg(seed);

            let node = node.fmap_into(|aa| {
                frontier.push_back(aa);
                // this is the sketchy bit, here - idx of pointed-to element
                elems.len() + frontier.len()
            });

            elems.push(node);
        }

        Self { elems }
    }
}

pub trait Recursive<A, O> {
    fn cata<F: FnMut(O) -> A>(self, alg: F) -> A;
}

impl<A, O, U: Functor<A, To = O, Unwrapped = usize>> Recursive<A, O> for RecursiveStruct<U> {
    fn cata<F: FnMut(O) -> A>(self, mut alg: F) -> A {
        let mut results: HashMap<usize, A> = HashMap::with_capacity(self.elems.len());

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it to avoid cloning owned data
                let node = node.fmap_into(|x| results.remove(&x).expect("node not in result map"));
                alg(node)
            };
            results.insert(idx, alg_res);
        }

        // assumes nonempty recursive structure
        results.remove(&0).unwrap()
    }
}

// TODO

// trait TryJoinFuture<'a> {
//     type Output;
//     type Error;
//     fn try_join(self) -> BoxFuture<'a, Result<Self::Output, Self::Error>>;
// }

// impl<'a, A: 'a + Send, E: 'a> TryJoinFuture<'a> for Expr<BoxFuture<'a, Result<A, E>>> {
//     type Output = Expr<A>;
//     type Error = E;

//     fn try_join(self) -> BoxFuture<'a, Result<Self::Output, Self::Error>> {
//         try_join_helper(self).boxed()
//     }
// }

// async fn try_join_helper<A, E>(e: Expr<BoxFuture<'_, Result<A, E>>>) -> Result<Expr<A>, E> {
//     match e {
//         Expr::Add(a, b) => {
//             let (a, b) = future::try_join(a, b).await?;
//             Ok(Expr::Add(a, b))
//         }
//         Expr::Sub(a, b) => {
//             let (a, b) = future::try_join(a, b).await?;
//             Ok(Expr::Sub(a, b))
//         }

//         Expr::Mul(a, b) => {
//             let (a, b) = future::try_join(a, b).await?;
//             Ok(Expr::Mul(a, b))
//         }

//         Expr::LiteralInt(x) => Ok(Expr::LiteralInt(x)),
//         Expr::DatabaseRef(key) => Ok(Expr::DatabaseRef(key)),
//     }
// }

// pub trait AsyncRecursive<'a, A: Send + Sync + 'a, E: Send + 'a, O>: Recursive<A, O> {
//     fn cata_async<
//         F: Fn(O) -> BoxFuture<'a, Result<A, E>> + Send + Sync + 'a,
//     >(
//         self,
//         alg: F,
//     ) -> BoxFuture<'a, Result<A, E>>;
// }

// impl<'a, A, E, O, U> AsyncRecursive for RecursiveStruct<U>

// trait AsyncRecursive {
//         // HAHA HOLY SHIT THIS RULES IT WORKS IT WORKS IT WORKS, GET A POSTGRES TEST GOING BECAUSE THIS RULES
//     pub async fn cata_async<
//         'a,
//         A: Send + Sync + 'a,
//         E: Send + 'a,
//         F: Fn(Expr<A>) -> BoxFuture<'a, Result<A, E>> + Send + Sync + 'a,
//     >(
//         self,
//         alg: F,
//     ) -> Result<A, E> {
//         let execution_graph = self.cata(|e|
//             // NOTE: want to directly pass in fn but can't because borrow checker - not sure how to do this, causes spurious clippy warning
//             cata_async_helper(e,  |x| alg(x)));

//         execution_graph.await
//     }
// }

// // given an async fun, build an execution graph from cata async
// fn cata_async_helper<
//     'a,
//     A: Send + 'a,
//     E: 'a,
//     F: Fn(Expr<A>) -> BoxFuture<'a, Result<A, E>> + Send + Sync + 'a,
// >(
//     e: Expr<BoxFuture<'a, Result<A, E>>>,
//     f: F,
// ) -> BoxFuture<'a, Result<A, E>> {
//     async move {
//         let e = e.try_join().await?;
//         f(e).await
//     }
//     .boxed()
// }
