use std::sync::Arc;

#[cfg(feature = "backcompat")]
use recursion::Collapse;

use crate::functor::{AsRefF, Compose, Functor, FunctorExt, PartiallyApplied, ToOwnedF, FunctorRef, RefFunctor};

use core::fmt::Debug;

pub trait Base {
    // NOTE: we would like to have an assoc type Frame<X> here but there's no way of asserting
    //       that Frame<X> and Frame<PartiallyApplied>::Layer<X> are equiv
    type MappableFrame: Functor;
}

type BaseFunctorToken<B> = <B as Base>::MappableFrame;
type BaseFunctor<B, X> = <<B as Base>::MappableFrame as Functor>::Layer<X>;
type BaseFunctorBorrowed<'a, B, X> = <<B as Base>::MappableFrame as RefFunctor>::Layer<'a, X>;

pub trait Recursive: Base
where
    Self: Sized,
{
    fn into_layer(self) -> BaseFunctor<Self, Self>;
}

pub trait Corecursive: Base
where
    Self: Sized,
{
    // likely invokes clone? idk actually
    fn from_layer(x: BaseFunctor<Self, Self>) -> Self;
}

mod test {
    use crate::functor::RefFunctor;

    use super::*;
    enum E {
        A(Box<E>, Box<E>),
        X(String),
    }

    // owned stack frame for E
    enum EF<X> {
        A(X, X),
        X(String),
    }

    impl Functor for EF<PartiallyApplied> {
        type Layer<X> = EF<X>;

        fn fmap<F, A, B>(input: Self::Layer<A>, mut f: F) -> Self::Layer<B>
        where
            F: FnMut(A) -> B,
        {
            match input {
                EF::A(a, b) => EF::A(f(a), f(b)),
                EF::X(n) => EF::X(n),
            }
        }
    }

    impl Base for E {
        type MappableFrame = EF<PartiallyApplied>;
    }

    impl Recursive for E {
        fn into_layer(self) -> BaseFunctor<Self, Self> {
            match self {
                E::A(a, b) => EF::A(*a, *b),
                E::X(n) => EF::X(n),
            }
        }
    }


    // borrowed stack frame for E
    enum EFB<'a, X> {
        A(X, X),
        X(&'a str),
    }


    impl<'a> Functor for EFB<'a, PartiallyApplied> {
        type Layer<X> = EFB<'a, X>;

        fn fmap<F, A, B>(input: Self::Layer<A>, mut f: F) -> Self::Layer<B>
        where
            F: FnMut(A) -> B,
        {
            match input {
                EFB::A(a, b) => EFB::A(f(a), f(b)),
                EFB::X(n) => EFB::X(n),
            }
        }
    }

    impl<'a> Base for &'a E {
        type MappableFrame = EFB<'a, PartiallyApplied>;
    }

    impl<'a> Recursive for &'a E {
        fn into_layer(self) -> BaseFunctor<Self, Self> {
            match self {
                E::A(a, b) => EFB::A(a.as_ref(), b.as_ref()),
                E::X(n) => EFB::X(n),
            }
        }
    }
}

// // impl<'a, F: Functor> Recursive for &'a Fix<F> {
// //     type FunctorToken = BorrowedFunctor<'a, F>;

// //     fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
// //         todo!()
// //     }
// // }

// // struct BorrowedFunctor<'a, F>(PhantomData<&'a F>);

// // impl<'a, G: Functor> Functor for BorrowedFunctor<'a, G> {
// //     type Layer<X> = &'a G::Layer<&'a X>;

// //     fn fmap<F, A, B>(input: Self::Layer<A>, f: F) -> Self::Layer<B>
// //     where
// //         F: FnMut(A) -> B {
// //         todo!()
// //     }
// // }

// // impl<F: Functor + TraverseResult> Debug for Fix<F> where
// //   F::Layer<String>: Debug,
// // {
// //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// //         F::expand_and_collapse(self, |layer| layer.0, |layer| |fmt| {
// //             // TODO: instead of building a nested closure thing, could we thread the formatter through as we do the expand step?
// //             let layer = F::fmap(layer, |x| x(fmt));
// //             F::flatten(layer).map( |layer| {
// //                 format!("{:?}")
// //             })
// //         })
// //         f.debug_tuple("Fix").field(&self.0).finish()
// //     }
// // }

// // impl<F: Functor> Deref for Fix<F> {
// //     type Target = &F::Layer<&Self>;

// //     fn deref(&self) -> &Self::Target {
// //         F.
// //     }
// // }

// // note to future me:
// // ok so - the AsRefF is just about being able to grab a _borrowed_ functor
// // that we can use for the traversal and the ToOwnedF is about being able to turn
// // those borrowed functor frames back into something owned (via clone)
// // stated differently: cloning a recursive structure is just round tripping through
// // ref form back into owned form - recursively descending ref's and cloning to rebuild
// impl<F: AsRefF> Clone for Fix<F>
// where
//     for<'a> F::RefFunctor<'a>: ToOwnedF<OwnedFunctor = F>,
// {
//     fn clone(&self) -> Self {
//         <F::RefFunctor<'_>>::expand_and_collapse(
//             self,
//             |x| F::as_ref(x.as_ref()),
//             |x| Fix::new(<F::RefFunctor<'_>>::to_owned(x)),
//         )
//     }
// }

// // ok I feel like god is talking to me, and is saying yo: have Fix be a fucking projection from some base type
// // like fucking do it this is insane. ok. yes.
// // impl<F: EqF> PartialEq for Fix<F>
// // {
// //     fn eq(&self, other: &Self) -> bool {
// //         type Func = Compose<Option<PartiallyApplied>, Compose<F::RefFunctor<'_>, PairFunctor>>;
// //         <Func as Functor>::expand_and_collapse(
// //             Some(F::pair_if_eq(self, other)),
// //             |x| x.map(|(a,b)| F::pair_if_eq(a, b)),
// //             |x| match x {
// //                 None => false,
// //                 Some(x) => {
// //                     let mut bools = Vec::new();
// //                     <F as AsRefF>::fmap(x, f)
// //                 }

// //             }
// //         )
// //     }
// // }

// // impl<F: AsRefF> PartialEq for Fix<F>
// // where
// //     for<'a> <F::RefFunctor<'a> as Functor>::Layer<bool>: Eq,
// // {
// //     // fn assert_receiver_is_total_eq(&self) {}

// //     fn eq(&self, other: &Self) -> bool {
// //         // wait fuck this doesn't work
// //         <Paired<F::RefFunctor<'_>>>::expand_and_collapse(
// //             (self, other),
// //             |(a, b)| (F::as_ref(a.as_ref()), F::as_ref(b.as_ref())),
// //             |_| a == b,
// //         )
// //     }
// // }

// // TODO: mb this just doesn't exist? this is janky af
// impl<F: for<'a> AsRefF<RefFunctor<'a> = G>, G: Functor> Debug for Fix<F>
// where
//     <G as Functor>::Layer<String>: std::fmt::Display,
// {
//     // TODO: thread actual fmt'r through instead of just repeatedly constructing strings, but eh - is fine for now
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let s = G::expand_and_collapse(
//             self,
//             |x: &Self| -> <G as Functor>::Layer<&Self> { F::as_ref(x.as_ref()) },
//             |layer: <G as Functor>::Layer<String>| -> String { format!("{}", layer) },
//         );
//         f.write_str(&s)
//     }
// }

// I love Fix but it scares the normies, leave it out (or in a submodule) for now

pub fn into_fix<X: Recursive>(x: X) -> Fix<BaseFunctorToken<X>> {
    BaseFunctorToken::<X>::expand_and_collapse(x, X::into_layer, Fix::new)
}

// pub fn from_fix<X: Corecursive>(x: Fix<X::FunctorToken>) -> X {
//     Fix::<X::FunctorToken>::fold_recursive(x, X::from_layer)
// }

/// heap allocated fix point of some Functor
#[derive(Debug)]
pub struct Fix<F: Functor>(pub Box<F::Layer<Fix<F>>>);

impl<F: Functor> Fix<F> {
    pub fn as_ref(&self) -> &F::Layer<Self> {
        self.0.as_ref()
    }
}

impl<F: Functor> Fix<F> {
    pub fn new(x: F::Layer<Self>) -> Self {
        Self(Box::new(x))
    }
}

impl<F: Functor> Base for Fix<F> {
    type MappableFrame = F;
}

// // recursing over a fix point structure is free
impl<F: Functor> Recursive for Fix<F> {
    fn into_layer(self) -> BaseFunctor<Self, Self> {
        *self.0
    }
}

// same for corecursion
impl<F: Functor> Corecursive for Fix<F> {
    fn from_layer(x: BaseFunctor<Self, Self>) -> Self {
        Fix::new(x)
    }
}

// // note could mb have another name for fold_recursive for borrowed data? would make API cleaner mb
// impl<'a, F: Functor + AsRefF> Recursive for &'a Fix<F> {
//     type FunctorToken = F::RefFunctor<'a>;

//     fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
//         F::as_ref(self.as_ref())
//     }
// }

// // TODO: futumorphism to allow for partial non-async expansion? yes! but (I think) needs to be erased for collapse phase
// // TODO: b/c at that point there's no need for that info..

// pub struct WithContext<R: Recursive>(pub R);

// impl<R: Recursive + Copy> Recursive for WithContext<R> {
//     type FunctorToken = Compose<R::FunctorToken, (R, PartiallyApplied)>;

//     fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
//         let layer = R::into_layer(self.0);
//         R::FunctorToken::fmap(layer, move |wrapped| (wrapped, WithContext(wrapped)))
//     }
// }

// pub struct PartialExpansion<R: Recursive> {
//     pub wrapped: R,
//     #[allow(clippy::type_complexity)]
//     pub f: Arc<
//         // TODO: probably doesn't need to be an arc but (shrug emoji)
//         dyn Fn(
//             <<R as Recursive>::FunctorToken as Functor>::Layer<R>,
//         ) -> <<R as Recursive>::FunctorToken as Functor>::Layer<Option<R>>,
//     >,
// }

// impl<R: Recursive> Recursive for PartialExpansion<R> {
//     type FunctorToken = Compose<R::FunctorToken, Option<PartiallyApplied>>;

//     fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
//         let partially_expanded = (self.f)(self.wrapped.into_layer());
//         Self::FunctorToken::fmap(partially_expanded, move |wrapped| PartialExpansion {
//             wrapped,
//             f: self.f.clone(),
//         })
//     }
// }

pub trait CorecursiveExt: Corecursive {
    fn unfold_recursive<In>(
        input: In,
        expand_layer: impl FnMut(In) -> BaseFunctor<Self, In>,
    ) -> Self;
}

impl<X> CorecursiveExt for X
where
    X: Corecursive,
{
    fn unfold_recursive<In>(
        input: In,
        expand_layer: impl FnMut(In) -> BaseFunctor<Self, In>,
    ) -> Self {
        BaseFunctorToken::<Self>::expand_and_collapse(input, expand_layer, Self::from_layer)
    }
}

pub trait RecursiveExt: Recursive {
    fn fold_recursive<Out>(self, collapse_layer: impl FnMut(BaseFunctor<Self, Out>) -> Out) -> Out;
}

impl<X> RecursiveExt for X
where
    X: Recursive,
{
    fn fold_recursive<Out>(self, collapse_layer: impl FnMut(BaseFunctor<Self, Out>) -> Out) -> Out {
        BaseFunctorToken::<Self>::expand_and_collapse(self, Self::into_layer, collapse_layer)
    }
}

// #[cfg(feature = "backcompat")]
// struct CollapseViaRecursive<X>(X);

// #[cfg(feature = "backcompat")]
// impl<Out, R: RecursiveExt> Collapse<Out, <<R as Recursive>::FunctorToken as Functor>::Layer<Out>>
//     for CollapseViaRecursive<R>
// {
//     fn collapse_layers<F: FnMut(<<R as Recursive>::FunctorToken as Functor>::Layer<Out>) -> Out>(
//         self,
//         collapse_layer: F,
//     ) -> Out {
//         self.0.fold_recursive(collapse_layer)
//     }
// }
