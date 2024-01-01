use std::marker::PhantomData;

use bumpalo::Bump;

/// A single 'frame' containing values that can be mapped over via `map_frame`.
///
/// # Motivation
///
/// Generally speaking, you won't use this trait yourself. It's used by the internal plumbing of
/// [`crate::Collapsible`] and [`crate::Expandable`] to implement recursive traversals.
///
/// # Implementing this trait
///
/// This trait is usually implemented for some marker token, because rust does not
/// allow for implementing a trait for a partially applied type. That is, we can implement
/// a trait for `Option<usize>` but we can't implement a trait for just `Option`, because
/// `Option` is a partially applied type.
///
/// For this reason, a common convention is to implement this trait using the uninhabited
///  [`PartiallyApplied`] enum marker, eg
///
/// ```rust
/// # use recursion::{MappableFrame, PartiallyApplied};
/// # #[derive(Debug, PartialEq, Eq)]
/// enum MyOption<A> {
///     Some(A),
///     None,
/// }
///
/// impl MappableFrame for MyOption<PartiallyApplied> {
///     type Frame<X> = MyOption<X>;
///
///     fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
///         match input {
///             MyOption::Some(x) => MyOption::Some(f(x)),
///             MyOption::None => MyOption::None,
///         }
///     }
/// }
/// ```
///
/// # Use
///
/// Here's what mapping over a `MyOption` frame looks like in action:
/// ```rust
/// # use recursion::{MappableFrame, PartiallyApplied};
/// # #[derive(Debug, PartialEq, Eq)]
/// # enum MyOption<A> {
/// #     Some(A),
/// #     None,
/// # }
/// #
/// # impl MappableFrame for MyOption<PartiallyApplied> {
/// #     type Frame<X> = MyOption<X>;
/// #
/// #     fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
/// #         match input {
/// #             MyOption::Some(x) => MyOption::Some(f(x)),
/// #             MyOption::None => MyOption::None,
/// #         }
/// #     }
/// # }
/// let frame = MyOption::Some(1);
/// let mapped_frame = MyOption::<PartiallyApplied>::map_frame(frame, |n| n + 10);
///
/// assert_eq!(mapped_frame, MyOption::Some(11));
/// ```
pub trait MappableFrame {
    /// the frame type that is mapped over by `map_frame`
    type Frame<X>;

    /// Apply some function `f` to each element inside a frame
    fn map_frame<A, B>(input: Self::Frame<A>, f: impl FnMut(A) -> B) -> Self::Frame<B>;
}

pub trait MappableFrameRefExt: MappableFrame {
    type I<'a, A: 'a>: Iterator<Item = &'a A>;

    // todo idk rename or whatever. may expand later.
    fn visit_subnodes<'a, A: 'a>(input: &'a Self::Frame<A>) -> Self::I<'a, A>;
}

/// "An uninhabited type used to define [`MappableFrame`] instances for partially-applied types."
///
/// For example: the MappableFrame instance for `MyFrame<A>` cannot be written over the
/// partially-applied type `MyFrame`, so instead we write it over `MyFrame<PartiallyApplied>`
#[derive(Clone, Debug)]
pub enum PartiallyApplied {}

/// This function generates a stack machine for some frame `F::Frame`,
/// expanding some seed value `Seed` into frames via a function `Seed -> Frame<Seed>`
/// and collapsing those values via a function `Frame<Out> -> Out`.
///
/// This function performs a depth-first traversal, expanding and collapsing each branch in turn
///
/// This function is stack safe (it does not use the call stack), but it
/// does use an internal stack data structure and is thus, technically,
/// susceptible to stack overflows if said stack expands
pub fn expand_and_collapse<F: MappableFrame, Seed, Out>(
    seed: Seed,
    mut expand_frame: impl FnMut(Seed) -> F::Frame<Seed>,
    mut collapse_frame: impl FnMut(F::Frame<Out>) -> Out,
) -> Out {
    enum State<Seed, CollapsibleInternal> {
        Expand(usize, Seed),
        Collapse(usize, CollapsibleInternal),
    }

    let mut vals: Vec<Option<Out>> = vec![None];
    let mut stack = vec![State::Expand(0, seed)];

    while let Some(item) = stack.pop() {
        match item {
            State::Expand(val_idx, seed) => {
                let node = expand_frame(seed);
                let mut seeds = Vec::new();
                let node = F::map_frame(node, |seed| {
                    vals.push(None);
                    let idx = vals.len() - 1;
                    seeds.push(State::Expand(idx, seed));
                    idx
                });

                stack.push(State::Collapse(val_idx, node));
                stack.extend(seeds);
            }
            State::Collapse(val_idx, node) => {
                let node = F::map_frame(node, |k| vals[k].take().unwrap());
                vals[val_idx] = Some(collapse_frame(node));
            }
        };
    }
    vals[0].take().unwrap()
}

/// This function generates a fallible stack machine for some frame `F::Frame`,
/// expanding some seed value `Seed` into frames via a function `Seed -> Result<Frame<Seed>, E>`
/// and collapsing those values via a function `Frame<Out> -> Result<Out, E>`.
///
/// This function performs a depth-first traversal, expanding and collapsing each branch in turn
///
/// This function is stack safe (it does not use the call stack), but it
/// does use an internal stack data structure and is thus, technically,
/// susceptible to stack overflows if said stack expands
pub fn try_expand_and_collapse<F: MappableFrame, Seed, Out, E>(
    seed: Seed,
    mut expand_frame: impl FnMut(Seed) -> Result<F::Frame<Seed>, E>,
    mut collapse_frame: impl FnMut(F::Frame<Out>) -> Result<Out, E>,
) -> Result<Out, E> {
    enum State<Seed, CollapsibleInternal> {
        Expand(usize, Seed),
        Collapse(usize, CollapsibleInternal),
    }

    let mut vals: Vec<Option<Out>> = vec![None];
    let mut stack = vec![State::Expand(0, seed)];

    while let Some(item) = stack.pop() {
        match item {
            State::Expand(val_idx, seed) => {
                let node = expand_frame(seed)?;
                let mut seeds = Vec::new();
                let node = F::map_frame(node, |seed| {
                    vals.push(None);
                    let idx = vals.len() - 1;
                    seeds.push(State::Expand(idx, seed));
                    idx
                });

                stack.push(State::Collapse(val_idx, node));
                stack.extend(seeds);
            }
            State::Collapse(val_idx, node) => {
                let node = F::map_frame(node, |k| vals[k].take().unwrap());
                vals[val_idx] = Some(collapse_frame(node)?);
            }
        };
    }
    Ok(vals[0].take().unwrap())
}

// EXPERIMENTAL BULLSHIT, we gonna just use fix point here because it's fun and also optimal-ish

/*
NOTE: I don't need to impl this structure to do the thing


NOTE: HOWEVER! I DO NEED THIS TO PROPERLY CLONE A FIX,
      otherwise it's all kinda shit (recursive traversal vs clone a flat vec)
 */

// struct FixRef();

// struct Fix<F: MappableFrame> {
//     root: FixRef,
//     nodes: Vec<F::Frame<FixRef>>,
//    // arena: Bump, // NOTE: add this later, heap is fine to start
// }

// struct FixRef<'a, F: MappableFrame> {
//     root: FixRef,
//     nodes: F::Frame<FixRef>
// }

// impl<F: MappableFrame> AsRef for Fix<F> {

// }

struct Generator<F: MappableFrame, Seed, Gen> {
    stack: Vec<Seed>,
    expand_frame: Box<dyn FnMut(Seed) -> F::Frame<Seed>>,
    generate: Box<dyn FnMut(&F::Frame<Seed>) -> Gen>,
}

struct GeneratorRef<'a, F: MappableFrameRefExt + 'a, Gen> {
    stack: Vec<&'a Fix<F>>,
    generate: Box<dyn FnMut(&'a <F as MappableFrame>::Frame<Fix<F>>) -> Gen>,
}

impl<'a, F: MappableFrameRefExt, G> GeneratorRef<'a, F, G> {
    pub fn new(
        seed: &'a Fix<F>,
        generate: Box<dyn FnMut(&'a <F as MappableFrame>::Frame<Fix<F>>) -> G>,
    ) -> Self {
        Self {
            stack: vec![&seed],
            generate,
        }
    }
}

impl<F: MappableFrame, G> Generator<F, Fix<F>, G> {
    pub fn new(seed: Fix<F>, generate: Box<dyn FnMut(&F::Frame<Fix<F>>) -> G>) -> Self {
        Self {
            stack: vec![seed],
            expand_frame: Box::new(|s| *s.0),
            generate,
        }
    }
}

impl<F: MappableFrame, S, G> Iterator for Generator<F, S, G> {
    type Item = G;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(seed) = self.stack.pop() {
            let frame = (self.expand_frame)(seed);
            let gen = (self.generate)(&frame);
            F::map_frame(frame, |s| self.stack.push(s));
            Some(gen)
        } else {
            None
        }
    }
}

impl<'a, F: MappableFrameRefExt, G> Iterator for GeneratorRef<'a, F, G> {
    type Item = G;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(Fix(frame)) = self.stack.pop() {
            let gen = (self.generate)(frame.as_ref());
            // TODO: decide if ref map frame gives internal value _or_ just a ref to it
            self.stack.extend(F::visit_subnodes(frame));
            Some(gen)
        } else {
            None
        }
    }
}

#[derive(Clone)]
// duplicate
pub struct Fix<F: MappableFrame>(pub Box<F::Frame<Fix<F>>>);

impl<F: MappableFrame> Fix<F> {
    pub fn new(x: F::Frame<Fix<F>>) -> Self {
        Self(Box::new(x))
    }

    pub fn into_expand_iter<G>(
        self,
        generate: impl FnMut(&F::Frame<Self>) -> G + 'static,
    ) -> Generator<F, Self, G> {
        Generator {
            stack: vec![self],
            expand_frame: Box::new(|s| *s.0),
            generate: Box::new(generate),
        }
    }
}

impl<F: MappableFrameRefExt> Fix<F> {
    pub fn expand_iter<'a, G>(
        self: &'a Self,
        generate: impl FnMut(&'a <F as MappableFrame>::Frame<Fix<F>>) -> G + 'static,
    ) -> GeneratorRef<'a, F, G> {
        GeneratorRef::new(self, Box::new(generate))
    }
}
// impl<F: MappableFrame, G> Generator<F, Fix<F>, G> {
// }

// pub struct FixRef<'a, F: MappableFrameRef + 'a>(pub &'a F::Frame<'a, FixRef<'a, F>>);

impl<V: core::fmt::Debug> core::fmt::Debug for Fix<Tree<V, PartiallyApplied>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut x = String::new();

        for n in self.0.elems.iter() {
            x.push_str(&format!("{:?}", n));
        }

        f.debug_tuple("Fix").field(&self.0.v).field(&x).finish()
    }
}

#[derive(Clone, Debug)]
struct Tree<V, Next> {
    v: V,
    elems: Vec<Next>,
}
impl<V> Tree<V, Fix<Tree<V, PartiallyApplied>>> {
    pub fn new(v: V, elems: Vec<Fix<Tree<V, PartiallyApplied>>>) -> Fix<Tree<V, PartiallyApplied>> {
        Fix::new(Tree { v, elems })
    }
}

impl<V> MappableFrame for Tree<V, PartiallyApplied> {
    type Frame<X> = Tree<V, X>;

    fn map_frame<A, B>(input: Self::Frame<A>, f: impl FnMut(A) -> B) -> Self::Frame<B> {
        Tree {
            elems: input.elems.into_iter().map(f).collect(),
            v: input.v,
        }
    }
}

impl<V> MappableFrameRefExt for Tree<V, PartiallyApplied> {
    type I<'a, A: 'a> = core::slice::Iter<'a, A>;

    fn visit_subnodes<'a, A: 'a>(input: &'a Self::Frame<A>) -> Self::I<'a, A> {
        input.elems.iter()
    }
}

#[test]
fn test_generator() {
    let t = Tree::new(
        "a.0".to_string(),
        vec![
            Tree::new(
                "b.1".to_string(),
                vec![Tree::new("c.2".to_string(), Vec::new())],
            ),
            Tree::new(
                "d.1".to_string(),
                vec![Tree::new("e.2".to_string(), Vec::new())],
            ),
        ],
    );

    let find_results: Vec<_> = t
        .expand_iter(|n| if n.v.contains('b') { Some(&n.v) } else { None })
        .filter_map(|x| x)
        .collect();

    assert_eq!(find_results, vec!["b.1"]);

    let elems: Vec<_> = t
        .into_expand_iter(|n| {
            println!("visit: {:?}", n);
            n.v.clone() // TODO: clone here is JANK, should be take-careable? I think.
        })
        .collect();

    assert_eq!(vec!["a.0", "d.1", "e.2", "b.1", "c.2"], elems);
}
