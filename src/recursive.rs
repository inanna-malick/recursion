use std::collections::HashMap;
use std::collections::VecDeque;

use futures::future::BoxFuture;
use futures::FutureExt;

/// Generic trait used to represent a recursive structure of some type F<usize>
pub struct RecursiveStruct<F> {
    // nonempty, in topological-sorted order
    elems: Vec<F>,
}

/// Support for recursion - folding a recursive structure into a single seed
pub trait Recursive<A, O> {
    fn cata<F: FnMut(O) -> A>(self, alg: F) -> A;
}

// TODO: filtered cata that has a pre-anything fn of, like, forall x F(x) -> Fx, so it can, like, drop directories or w/e by looking at 1 layer only

// answer to visitor pattern question (how to do some actions in before, some in after branches)
// my answer: do the 'before'/'filter' type stuff in ana, as the structure is built (not a great answer)

/// Support for corecursion - unfolding a recursive structure from a seed
pub trait CoRecursive<A, O> {
    fn ana<F: Fn(A) -> O>(a: A, coalg: F) -> Self;
}

pub trait CoRecursiveAsync<A, O> {
    fn ana_result_async<
        'a,
        E: Send + 'a,
        F: Fn(A) -> BoxFuture<'a, Result<O, E>> + Send + Sync + 'a,
    >(
        a: A,
        coalg: F,
    ) -> BoxFuture<'a, Result<Self, E>>
    where
        Self: Sized,
        A: Send + 'a;
}

impl<A, U, O: Functor<usize, Unwrapped = A, To = U>> CoRecursive<A, O> for RecursiveStruct<U> {
    fn ana<F: Fn(A) -> O>(a: A, coalg: F) -> Self {
        let mut frontier = VecDeque::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop_front() {
            let node = coalg(seed);

            let node = node.fmap_into(|aa| {
                frontier.push_back(aa);
                // idx of pointed-to element determined from frontier + elems size
                elems.len() + frontier.len()
            });

            elems.push(node);
        }

        Self { elems }
    }
}

impl<A, U: Send, O: Functor<usize, Unwrapped = A, To = U>> CoRecursiveAsync<A, O>
    for RecursiveStruct<U>
{
    fn ana_result_async<
        'a,
        E: Send + 'a,
        F: Fn(A) -> BoxFuture<'a, Result<O, E>> + Send + Sync + 'a,
    >(
        a: A,
        coalg: F,
    ) -> BoxFuture<'a, Result<Self, E>>
    where
        Self: Sized,
        U: Send,
        A: Send + 'a,
    {
        async move {
            let mut frontier = VecDeque::from([a]);
            let mut elems = vec![];

            // unfold to build a vec of elems while preserving topo order
            while let Some(seed) = frontier.pop_front() {
                let node = coalg(seed).await?;

                let node = node.fmap_into(|aa| {
                    frontier.push_back(aa);
                    // idx of pointed-to element determined from frontier + elems size
                    elems.len() + frontier.len()
                });

                elems.push(node);
            }

            Ok(Self { elems })
        }
        .boxed()
    }
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

pub trait Functor<B> {
    type Unwrapped;
    type To;
    /// fmap over an owned value. Sort of like 'into_iter()' except for arbitrary recursive structures
    fn fmap_into<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To;
}

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
