pub mod eval;
pub mod monomorphic;
pub mod naive;

use recursion::{
    map_layer::MapLayer,
    recursive_tree::{arena_eval::ArenaIndex, stack_machine_eval::StackMarker, RecursiveTree},
};
use recursion_schemes::functor::{Functor, PartiallyApplied};

/// Simple expression language with some operations on integers
#[derive(Debug, Clone, Copy)]
pub enum Expr<A> {
    Add(A, A),
    Sub(A, A),
    Mul(A, A),
    LiteralInt(i64),
}

impl Functor for Expr<PartiallyApplied> {
    type Layer<X> = Expr<X>;

    #[inline(always)]
    fn fmap<F, A, B>(input: Self::Layer<A>, mut f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B,
    {
        match input {
            Expr::Add(a, b) => Expr::Add(f(a), f(b)),
            Expr::Sub(a, b) => Expr::Sub(f(a), f(b)),
            Expr::Mul(a, b) => Expr::Mul(f(a), f(b)),
            Expr::LiteralInt(x) => Expr::LiteralInt(x),
        }
    }
}

mod example {
    use recursion_schemes::functor::*;
    use recursion_schemes::recursive::*;

    #[derive(Debug, Clone, Copy)]
    pub enum Expr<A> {
        Add(A, A),
        Sub(A, A),
        Mul(A, A),
        LiteralInt(i64),
    }

    impl Functor for Expr<PartiallyApplied> {
        type Layer<X> = Expr<X>;

        #[inline(always)]
        fn fmap<F, A, B>(input: Self::Layer<A>, mut f: F) -> Self::Layer<B>
        where
            F: FnMut(A) -> B,
        {
            match input {
                Expr::Add(a, b) => Expr::Add(f(a), f(b)),
                Expr::Sub(a, b) => Expr::Sub(f(a), f(b)),
                Expr::Mul(a, b) => Expr::Mul(f(a), f(b)),
                Expr::LiteralInt(x) => Expr::LiteralInt(x),
            }
        }
    }


    type AnnotationFunctor<T> = (T, PartiallyApplied);
    type ExprFunctor = Expr<PartiallyApplied>;

    type Depth = usize;

    type AnnotatedExprFunctor = Compose<AnnotationFunctor<Depth>, ExprFunctor>;

    impl AsRefF for ExprFunctor {
        type RefFunctor<'a> = Expr<PartiallyApplied>;

        fn as_ref<'a, A>(
        input: &'a <Self as Functor>::Layer<A>,
        ) -> <Self::RefFunctor<'a> as Functor>::Layer<&'a A> {
            match input {
                Expr::Add(a, b) => Expr::Add(a, b),
                Expr::Sub(a, b) => Expr::Sub(a, b),
                Expr::Mul(a, b) => Expr::Mul(a, b),
                Expr::LiteralInt(x) => Expr::LiteralInt(x),
            }
        }
    }

    fn tag_depth(e: Fix<ExprFunctor>) -> Fix<AnnotatedExprFunctor> {
        e.fold_recursive(|layer| {
            let mut max_depth = 0;
            let layer = ExprFunctor::fmap(
                layer,
                |Fix(annotated_subnode): Fix<AnnotatedExprFunctor>| {
                    let subnode_depth = (*annotated_subnode).0;
                    max_depth = max_depth.max(subnode_depth);
                    Fix::<AnnotatedExprFunctor>(annotated_subnode)
                },
            );
            Fix(Box::new((max_depth + 1, layer)))
        })
    }

    #[test]
    fn test_example() {
        let x = Fix::new(Expr::Add(
            Fix::new(Expr::LiteralInt(1)),
            Fix::new(Expr::Mul(
                Fix::new(Expr::LiteralInt(2)),
                Fix::new(Expr::LiteralInt(2)),
            )),
        ));

        let annotated = tag_depth(x);

        assert_eq!(annotated.as_ref().0, 3, "expr: {:?}", annotated);
    }
}

// impl JoinFuture for Expr<PartiallyApplied> {
//     type FunctorToken = Expr<PartiallyApplied>;

//     fn join_layer<A: Send + 'static>(
//         input: <<Self as JoinFuture>::FunctorToken as Functor>::Layer<BoxFuture<'static, A>>,
//     ) -> BoxFuture<'static, <<Self as JoinFuture>::FunctorToken as Functor>::Layer<A>> {
//         async {
//             use futures::future::join;
//             match input {
//                 Expr::Add(a, b) => {
//                     let (a, b) = join(a, b).await;
//                     Expr::Add(a, b)
//                 }
//                 Expr::Sub(a, b) => {
//                     let (a, b) = join(a, b).await;
//                     Expr::Sub(a, b)
//                 }
//                 Expr::Mul(a, b) => {
//                     let (a, b) = join(a, b).await;
//                     Expr::Mul(a, b)
//                 }
//                 Expr::LiteralInt(x) => Expr::LiteralInt(x),
//             }
//         }
//         .boxed()
//     }
// }

impl<A, B> MapLayer<B> for Expr<A> {
    type To = Expr<B>;
    type Unwrapped = A;

    #[inline(always)]
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            Expr::Add(a, b) => Expr::Add(f(a), f(b)),
            Expr::Sub(a, b) => Expr::Sub(f(a), f(b)),
            Expr::Mul(a, b) => Expr::Mul(f(a), f(b)),
            Expr::LiteralInt(x) => Expr::LiteralInt(x),
        }
    }
}

// this is, like, basically fine? - just usize and ()
impl<'a, A: Copy, B: 'a> MapLayer<B> for &'a Expr<A> {
    type To = Expr<B>;
    type Unwrapped = A;

    #[inline(always)]
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            Expr::Add(a, b) => Expr::Add(f(*a), f(*b)),
            Expr::Sub(a, b) => Expr::Sub(f(*a), f(*b)),
            Expr::Mul(a, b) => Expr::Mul(f(*a), f(*b)),
            Expr::LiteralInt(x) => Expr::LiteralInt(*x),
        }
    }
}

pub type DFSStackExpr = RecursiveTree<Expr<StackMarker>, StackMarker>;
pub type BlocAllocExpr = RecursiveTree<Expr<ArenaIndex>, ArenaIndex>;
