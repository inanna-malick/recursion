#[cfg(test)]
use proptest::prelude::*;
use std::collections::{HashMap, VecDeque};

// for blog post
// start with some examples of structures that are best represented as recursive
// - file tree, repository structure, language AST, expression languages as used, eg, to filter tests in nextest.
// state that we'll be working with a simple
// start with AST, boxed, show simple recursive function over expression.
// note that it has bad perf due to pointer chasing
// show more performant data storage impl: tree as vec with usize pointers.
// show next impl of Expr, with recursive links monomorphic-replaced with usize pointers
// implement (ugh, I know) recursive traversal over that - catamorphism with algorithm built in
// let's imagine we start with json instead of parsing the expr AST, much cleaner that way, conceptually
// go ana -> cata, explain how to build it, plus topo sort, before exaplaining how to consume it
// so, first - show parser for json expressions w/ boxed recursion, then show eval for same

// or actually no, that sucks - we don't want to parse json because it's long and boring
// use expr AST as in-memory repr, just write those out and be like, look, it good. I'd be basically doing that

// this is a post about writing elegant and performant recursive algorithms in rust.
// (It makes heavy use of a pattern from haskell called recursion schemes, but you don't need to know anything about that)
// I've used it to implement a nontrivial proof of concept - if you look in the top-level examples directory there's a
// small but functional grep app that uses async IO, handles failure cases, etc. We're not going to start with that, though.

// We're going to start with a simple expression language: addition, subtraction, multiplication, just enough to illustrate some concepts.
// This is a naive representation of a recursive expression language that uses boxed pointers to handle the recursive case.
// You've probably seen something like this before, but if not, it's just a way to represent simple arithmetic, expressions like `1 + 2 * 3 - 4`

#[derive(Debug, Clone)]
pub enum ExprBoxed {
    Add {
        a: Box<ExprBoxed>,
        b: Box<ExprBoxed>,
    },
    Sub {
        a: Box<ExprBoxed>,
        b: Box<ExprBoxed>,
    },
    Mul {
        a: Box<ExprBoxed>,
        b: Box<ExprBoxed>,
    },
    LiteralInt {
        literal: i64,
    },
}

impl ExprBoxed {
    pub fn eval(&self) -> i64 {
        match &self {
            ExprBoxed::Add { a, b } => a.eval() + b.eval(),
            ExprBoxed::Sub { a, b } => a.eval() - b.eval(),
            ExprBoxed::Mul { a, b } => a.eval() * b.eval(),
            ExprBoxed::LiteralInt { literal } => *literal,
        }
    }
}

/// Simple expression language with some operations on integers
#[derive(Debug, Clone, Copy)]
pub enum ExprLayer<A> {
    Add { a: A, b: A },
    Sub { a: A, b: A },
    Mul { a: A, b: A },
    LiteralInt { literal: i64 },
}

#[derive(Eq, Hash, PartialEq)]
pub struct ExprIdx(usize);
impl ExprIdx {
    fn head() -> Self {
        ExprIdx(0)
    }
}

pub struct Expr {
    // nonempty, in topological-sorted order. for every node `n`, all of `n`'s child nodes have vec indices greater than that of n
    elems: Vec<ExprLayer<ExprIdx>>,
}

// the problem here is that this is harder to read - we don't want to construct these by hand, because it would be tedious and error prone.
// fortunately we can just create them from boxed expressions.

// Here's how to do so: we take a boxed expression, and generate a single layer of structure from it, repeatedly generateing layers until all the boxed expressions are converted to this repr

impl Expr {
    fn generate_from_boxed_inline(a: &ExprBoxed) -> Self {
        let mut frontier: VecDeque<&ExprBoxed> = VecDeque::new();
        let mut elems = vec![];

        fn push_to_frontier<'a>(
            elems: &Vec<ExprLayer<ExprIdx>>,
            frontier: &mut VecDeque<&'a ExprBoxed>,
            a: &'a ExprBoxed,
        ) -> ExprIdx {
            frontier.push_back(a);
            // idx of pointed-to element determined from frontier + elems size
            ExprIdx(elems.len() + frontier.len())
        }

        push_to_frontier(&elems, &mut frontier, a);

        // generate to build a vec of elems while preserving topo order
        while let Some(seed) = { frontier.pop_front() } {
            let node = match seed {
                ExprBoxed::Add { a, b } => {
                    let a = push_to_frontier(&elems, &mut frontier, a);
                    let b = push_to_frontier(&elems, &mut frontier, b);
                    ExprLayer::Add { a, b }
                }
                ExprBoxed::Sub { a, b } => {
                    let a = push_to_frontier(&elems, &mut frontier, a);
                    let b = push_to_frontier(&elems, &mut frontier, b);
                    ExprLayer::Sub { a, b }
                }
                ExprBoxed::Mul { a, b } => {
                    let a = push_to_frontier(&elems, &mut frontier, a);
                    let b = push_to_frontier(&elems, &mut frontier, b);
                    ExprLayer::Mul { a, b }
                }
                ExprBoxed::LiteralInt { literal } => {
                    // no more nodes to explore
                    ExprLayer::LiteralInt { literal: *literal }
                }
            };

            elems.push(node);
        }

        Self { elems }
    }
}

impl Expr {
    fn generate_from_boxed_with_map(seed: &ExprBoxed) -> Self {
        let mut frontier: VecDeque<&ExprBoxed> = VecDeque::from([seed]);
        let mut elems = vec![];

        // generate to build a vec of elems while preserving topo order
        while let Some(seed) = { frontier.pop_front() } {
            let node = match seed {
                ExprBoxed::Add { a, b } => ExprLayer::Add { a, b },
                ExprBoxed::Sub { a, b } => ExprLayer::Sub { a, b },
                ExprBoxed::Mul { a, b } => ExprLayer::Mul { a, b },
                ExprBoxed::LiteralInt { literal } => ExprLayer::LiteralInt { literal: *literal },
            };
            let node = node.map(|seed| {
                frontier.push_back(seed);
                // idx of pointed-to element determined from frontier + elems size
                ExprIdx(elems.len() + frontier.len())
            });

            elems.push(node);
        }

        Self { elems }
    }
}

// here we start with a single seed and generate it into an expression tree.

// (NOTE: let's not use the terms fold/generate yet)
// it's not exactly elegant, right? The block [here] defines a single recursive step, which builds a single layer of Expr<&ExprBoxed> structure from an &ExprBoxed seed value,
// but it's surrounded with a bunch of bookkeeping boilerplate that handles combining the layers to build a vec of `Expr<usize>`
// Don't worry, we have a fix for this that we'll get into momentarily.

// before we get into that, let's look at what evaluating a recursive structure into a single value looks like:

impl Expr {
    fn eval_inline(self) -> i64 {
        use std::mem::MaybeUninit;

        let mut results = std::iter::repeat_with(|| MaybeUninit::<i64>::uninit())
            .take(self.elems.len())
            .collect::<Vec<_>>();

        fn get_result_unsafe(results: &mut Vec<MaybeUninit<i64>>, idx: ExprIdx) -> i64 {
            unsafe {
                let maybe_uninit =
                    std::mem::replace(results.get_unchecked_mut(idx.0), MaybeUninit::uninit());
                maybe_uninit.assume_init()
            }
        }

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it, also we know it's there so unsafe is fine
                match node {
                    ExprLayer::Add { a, b } => {
                        let a = get_result_unsafe(&mut results, a);
                        let b = get_result_unsafe(&mut results, b);
                        a + b
                    }
                    ExprLayer::Sub { a, b } => {
                        let a = get_result_unsafe(&mut results, a);
                        let b = get_result_unsafe(&mut results, b);
                        a - b
                    }
                    ExprLayer::Mul { a, b } => {
                        let a = get_result_unsafe(&mut results, a);
                        let b = get_result_unsafe(&mut results, b);
                        a * b
                    }
                    ExprLayer::LiteralInt { literal } => literal,
                }
            };
            results[idx].write(alg_res);
        }

        unsafe {
            let maybe_uninit =
                std::mem::replace(results.get_unchecked_mut(0), MaybeUninit::uninit());
            maybe_uninit.assume_init()
        }
    }
}

// NOTE NOTE NOTE: introduce fmap FIRST, show how it can reduce duplicated code
impl<A> ExprLayer<A> {
    #[inline(always)]
    fn map<B, F: FnMut(A) -> B>(self, mut f: F) -> ExprLayer<B> {
        match self {
            ExprLayer::Add { a, b } => ExprLayer::Add { a: f(a), b: f(b) },
            ExprLayer::Sub { a, b } => ExprLayer::Sub { a: f(a), b: f(b) },
            ExprLayer::Mul { a, b } => ExprLayer::Mul { a: f(a), b: f(b) },
            ExprLayer::LiteralInt { literal } => ExprLayer::LiteralInt { literal },
        }
    }
}

impl Expr {
    fn eval_inline_fmap(self) -> i64 {
        use std::mem::MaybeUninit;

        let mut results = std::iter::repeat_with(|| MaybeUninit::<i64>::uninit())
            .take(self.elems.len())
            .collect::<Vec<_>>();

        fn get_result_unsafe(results: &mut Vec<MaybeUninit<i64>>, idx: ExprIdx) -> i64 {
            unsafe {
                let maybe_uninit =
                    std::mem::replace(results.get_unchecked_mut(idx.0), MaybeUninit::uninit());
                maybe_uninit.assume_init()
            }
        }

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let node = node.map(|idx| unsafe {
                let maybe_uninit =
                    std::mem::replace(results.get_unchecked_mut(idx.0), MaybeUninit::uninit());
                maybe_uninit.assume_init()
            });

            let alg_res = match node {
                ExprLayer::Add { a, b } => a + b,
                ExprLayer::Sub { a, b } => a - b,
                ExprLayer::Mul { a, b } => a * b,
                ExprLayer::LiteralInt { literal } => literal,
            };
            results[idx].write(alg_res);
        }

        unsafe {
            let maybe_uninit =
                std::mem::replace(results.get_unchecked_mut(0), MaybeUninit::uninit());
            maybe_uninit.assume_init()
        }
    }
}

// here we fold up the expression tree from the leaves to the root, evaluating it one layer at a time and storing the results in a hashmap until they are used.
// Everything is stored in a vec, so we don't need to worry about the overhead of traversing a bunch of pointers

// ok, still not elegant. Once again, the logic of _how_ we fold layers of recursive structure (Expr<i64>) into a single value (i64)
// is interleaved with a bunch of boilerplate that handles the actual mechanics of recursion.

// RECURSION SCHEMES

// The key idea here, which is taken entirely from recursion schemes, is to _separate_ the mechanism of recursion from the logic of recursion.
// Let's see what that looks like:

impl Expr {
    fn fold<A: std::fmt::Debug, F: FnMut(ExprLayer<A>) -> A>(self, mut alg: F) -> A {
        use std::mem::MaybeUninit;

        let mut results = std::iter::repeat_with(|| MaybeUninit::<A>::uninit())
            .take(self.elems.len())
            .collect::<Vec<_>>();

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it, also we know it's there so unsafe is fine
                let node = node.map(|x| unsafe {
                    let maybe_uninit =
                        std::mem::replace(results.get_unchecked_mut(x.0), MaybeUninit::uninit());
                    maybe_uninit.assume_init()
                });
                alg(node)
            };
            results[idx].write(alg_res);
        }

        unsafe {
            let maybe_uninit = std::mem::replace(
                results.get_unchecked_mut(ExprIdx::head().0),
                MaybeUninit::uninit(),
            );
            maybe_uninit.assume_init()
        }
    }
}

// First, we have a generic representation of folding some structure into a single value - instead of folding an Expr<i64> into a single i64,
// we fold some Expr<A> into an A. The code looks pretty much the same as eval_inline, but it lets us factor out the mechanism of recursion.

impl Expr {
    pub fn eval(self) -> i64 {
        self.fold(|expr| match expr {
            ExprLayer::Add { a, b } => a + b,
            ExprLayer::Sub { a, b } => a - b,
            ExprLayer::Mul { a, b } => a * b,
            ExprLayer::LiteralInt { literal } => literal,
        })
    }
}

// Having done so, we can then write a really clean elegant implementation of eval by folding with pretty much the same logic as we had in eval_inline
// This function is clean and doesn't contain any boilerplate, which means there's less room for bugs. In fact, it actually contains slightly less boilerplate than
// the eval function we wrote for ExprBoxed::eval because it doesn't have to handle recursively calling itself. Even better, it's extremely performant -
// instead of following a bunch of pointers it just calls into_iter on a vector and consumes the resulting iterator (TODO: numbers here)

impl Expr {
    fn generate<A, F: Fn(A) -> ExprLayer<A>>(a: A, coalg: F) -> Self {
        let mut frontier = VecDeque::from([a]);
        let mut elems = vec![];

        // generate to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop_front() {
            let node = coalg(seed);

            let node = node.map(|aa| {
                frontier.push_back(aa);
                // idx of pointed-to element determined from frontier + elems size
                ExprIdx(elems.len() + frontier.len())
            });

            elems.push(node);
        }

        Self { elems }
    }
}

// Second, we have a generic representation of generateing some structure from a single value - instead of generateing
// a single layer of Expr<&ExprBoxed> structure from an &ExprBoxed seed value,
// we generate some A into an Expr<A>. As with fold, the code looks pretty much the same as from_ast_inline, just with the
// specific generate logic factored out

impl Expr {
    pub fn generate_from_boxed(ast: &ExprBoxed) -> Self {
        Self::generate(ast, |seed| match seed {
            ExprBoxed::Add { a, b } => ExprLayer::Add { a, b },
            ExprBoxed::Sub { a, b } => ExprLayer::Sub { a, b },
            ExprBoxed::Mul { a, b } => ExprLayer::Mul { a, b },
            ExprBoxed::LiteralInt { literal } => ExprLayer::LiteralInt { literal: *literal },
        })
    }
}

// generate a bunch of expression trees and evaluate them via each method
#[cfg(test)]
proptest! {
    #[test]
    fn expr_eval(boxed_expr in arb_expr()) {
        let eval_boxed = boxed_expr.eval();

        let eval_inlined = Expr::generate_from_boxed_inline(&boxed_expr).eval_inline();
        let eval_via_fold = Expr::generate_from_boxed(&boxed_expr).eval();

        assert_eq!(eval_boxed, eval_inlined);
        assert_eq!(eval_inlined, eval_via_fold);
    }
}

#[cfg(test)]
pub fn arb_expr() -> impl Strategy<Value = ExprBoxed> {
    let leaf = any::<i8>().prop_map(|x| ExprBoxed::LiteralInt { literal: x as i64 });
    leaf.prop_recursive(
        8,   // 8 levels deep
        256, // Shoot for maximum size of 256 nodes
        10,  // We put up to 10 items per collection
        |inner| {
            prop_oneof![
                (inner.clone(), inner.clone()).prop_map(|(a, b)| ExprBoxed::Add {
                    a: Box::new(a),
                    b: Box::new(b)
                }),
                (inner.clone(), inner.clone()).prop_map(|(a, b)| ExprBoxed::Sub {
                    a: Box::new(a),
                    b: Box::new(b)
                }),
                (inner.clone(), inner).prop_map(|(a, b)| ExprBoxed::Mul {
                    a: Box::new(a),
                    b: Box::new(b)
                }),
            ]
        },
    )
}

// TODO: benchmark a bunch of expr eval operations on the boxed vs. unboxed versions - will make for impressive numbers <- TODO TODO TODO
