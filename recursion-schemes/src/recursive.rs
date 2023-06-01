#[cfg(feature = "backcompat")]
use recursion::Collapse;

use crate::functor::{
    AsRefF, Compose, Functor, FunctorExt, PartiallyApplied, ToOwnedF,
};

use core::fmt::Debug;

pub trait Recursive
where
    Self: Sized,
{
    type MappableFrame: Functor;
    fn into_layer(self) -> <Self::MappableFrame as Functor>::Layer<Self>;
}

pub trait Corecursive
where
    Self: Sized,
{
    type MappableFrame: Functor;
    // likely invokes clone? idk actually
    fn from_layer(x: <Self::MappableFrame as Functor>::Layer<Self>) -> Self;
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

    fn fmap<A, B>(input: Self::Layer<A>, mut f: impl FnMut(A) -> B) -> Self::Layer<B>
        {
            match input {
                EF::A(a, b) => EF::A(f(a), f(b)),
                EF::X(n) => EF::X(n),
            }
        }
    }

    impl Recursive for E {
        type MappableFrame = EF<PartiallyApplied>;

        fn into_layer(self) -> <Self::MappableFrame as Functor>::Layer<Self> {
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

    fn fmap<A, B>(input: Self::Layer<A>, mut f: impl FnMut(A) -> B) -> Self::Layer<B>
        {
            match input {
                EFB::A(a, b) => EFB::A(f(a), f(b)),
                EFB::X(n) => EFB::X(n),
            }
        }
    }

    impl<'a> Recursive for &'a E {
        fn into_layer(self) -> <Self::MappableFrame as Functor>::Layer<Self> {
            match self {
                E::A(a, b) => EFB::A(a.as_ref(), b.as_ref()),
                E::X(n) => EFB::X(n),
            }
        }

        type MappableFrame = EFB<'a, PartiallyApplied>;
    }
}



// // note to future me:
// // ok so - the AsRefF is just about being able to grab a _borrowed_ functor
// // that we can use for the traversal and the ToOwnedF is about being able to turn
// // those borrowed functor frames back into something owned (via clone)
// // stated differently: cloning a recursive structure is just round tripping through
// // ref form back into owned form - recursively descending ref's and cloning to rebuild
impl<F: AsRefF> Clone for Fix<F>
where
    for<'a> F::RefFunctor<'a>: ToOwnedF<OwnedFunctor = F>,
{
    fn clone(&self) -> Self {
        <F::RefFunctor<'_>>::expand_and_collapse(
            self,
            |x| F::as_ref(x.as_ref()),
            |x| Fix::new(<F::RefFunctor<'_>>::to_owned(x)),
        )
    }
}

// I love Fix but it scares the normies, leave it out (or in a submodule) for now

pub fn into_fix<X: Recursive>(x: X) -> Fix<X::MappableFrame> {
    X::MappableFrame::expand_and_collapse(x, X::into_layer, Fix::new)
}

pub fn from_fix<X: Corecursive>(x: Fix<X::MappableFrame>) -> X {
    Fix::<X::MappableFrame>::fold_recursive(x, X::from_layer)
}

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

// // recursing over a fix point structure is free
impl<F: Functor> Recursive for Fix<F> {
    type MappableFrame = F;
    fn into_layer(self) -> <Self::MappableFrame as Functor>::Layer<Self> {
        *self.0
    }
}

// same for corecursion
impl<F: Functor> Corecursive for Fix<F> {
    type MappableFrame = F;
    fn from_layer(x: <Self::MappableFrame as Functor>::Layer<Self>) -> Self {
        Fix::new(x)
    }
}

// note could mb have another name for fold_recursive for borrowed data? would make API cleaner mb
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
        expand_layer: impl FnMut(In) -> <Self::MappableFrame as Functor>::Layer<In>,
    ) -> Self;
}

impl<X> CorecursiveExt for X
where
    X: Corecursive,
{
    fn unfold_recursive<In>(
        input: In,
        expand_layer: impl FnMut(In) -> <Self::MappableFrame as Functor>::Layer<In>,
    ) -> Self {
        Self::MappableFrame::expand_and_collapse(input, expand_layer, Self::from_layer)
    }
}

pub trait RecursiveExt: Recursive {
    fn fold_recursive<Out>(
        self,
        collapse_layer: impl FnMut(<Self::MappableFrame as Functor>::Layer<Out>) -> Out,
    ) -> Out;
}

impl<X> RecursiveExt for X
where
    X: Recursive,
{
    fn fold_recursive<Out>(
        self,
        collapse_layer: impl FnMut(<Self::MappableFrame as Functor>::Layer<Out>) -> Out,
    ) -> Out {
        Self::MappableFrame::expand_and_collapse(self, Self::into_layer, collapse_layer)
    }
}

#[cfg(feature = "backcompat")]
struct CollapseViaRecursive<X>(X);

#[cfg(feature = "backcompat")]
impl<Out, R: RecursiveExt> Collapse<Out, <<R as Recursive>::FunctorToken as Functor>::Layer<Out>>
    for CollapseViaRecursive<R>
{
    fn collapse_layers<F: FnMut(<<R as Recursive>::FunctorToken as Functor>::Layer<Out>) -> Out>(
        self,
        collapse_layer: F,
    ) -> Out {
        self.0.fold_recursive(collapse_layer)
    }
}
