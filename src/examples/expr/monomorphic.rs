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

// evaluation is pretty simple - addition, subtraction, multiplication (todo run on 1 + 2 * 3 - 4).
// This function provides an elegant and readable representation of a recursive algorithm, but it will fail with a stack overflow if called
// a sufficiently large expression - we're not likely to hit that case here, but this is a real problem when working with larger recursive data structures
// like file trees or version control repository state.

// Another problem with this function is that it requires pointer chasing - each recursion into a boxed value requires traversing a pointer,
// which means we can't take advantage of cache locality - there's no guarantee that all these boxed pointers live in the same region of memory.
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

// We're going to sketch out a more performant expression language, using a Vec of values (guaranteeing memory locality) with boxed pointers replaced
// with usize indices pointing into our vector.

/// Simple expression language with some operations on integers
#[derive(Debug, Clone, Copy)]
pub enum Expr<A> {
    Add { a: A, b: A },
    Sub { a: A, b: A },
    Mul { a: A, b: A },
    LiteralInt { literal: i64 },
}
pub struct RecursiveExpr {
    // nonempty, in topological-sorted order. for every node `n`, all of `n`'s child nodes have vec indices greater than that of n
    elems: Vec<Expr<usize>>,
}

// we're also going to define a function to map from one type of Expr to another, for convenience - nothing complex, it just applies a function to each 'A', sort of like mapping over an Option or an Iterator
impl<A> Expr<A> {
    fn map<B, F: FnMut(A) -> B>(self, mut f: F) -> Expr<B> {
        match self {
            Expr::Add { a, b } => Expr::Add { a: f(a), b: f(b) },
            Expr::Sub { a, b } => Expr::Sub { a: f(a), b: f(b) },
            Expr::Mul { a, b } => Expr::Mul { a: f(a), b: f(b) },
            Expr::LiteralInt { literal } => Expr::LiteralInt { literal },
        }
    }
}

// the problem here is that this is harder to read - we don't want to construct these by hand, because it would be tedious and error prone.
// fortunately we can just create them from boxed expressions.

// Here's how to do so: we take a boxed expression, and unfold a single layer of structure from it, repeatedly unfolding layers until all the boxed expressions are converted to this repr

impl RecursiveExpr {
    fn from_ast_inline(a: &ExprBoxed) -> Self {
        let mut frontier: VecDeque<&ExprBoxed> = VecDeque::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop_front() {
            let node = match seed {
                ExprBoxed::Add { a, b } => Expr::Add { a, b },
                ExprBoxed::Sub { a, b } => Expr::Sub { a, b },
                ExprBoxed::Mul { a, b } => Expr::Mul { a, b },
                ExprBoxed::LiteralInt { literal } => Expr::LiteralInt { literal: *literal },
            };

            let node = node.map(|aa| {
                frontier.push_back(aa);
                // idx of pointed-to element determined from frontier + elems size
                elems.len() + frontier.len()
            });

            elems.push(node);
        }

        Self { elems }
    }
}

// here we start with a single seed and unfold it into an expression tree.

// (NOTE: let's not use the terms fold/unfold yet)
// it's not exactly elegant, right? The block [here] defines a single recursive step, which builds a single layer of Expr<&ExprBoxed> structure from an &ExprBoxed seed value,
// but it's surrounded with a bunch of bookkeeping boilerplate that handles combining the layers to build a vec of `Expr<usize>`
// Don't worry, we have a fix for this that we'll get into momentarily.

// before we get into that, let's look at what evaluating a recursive structure into a single value looks like:

impl RecursiveExpr {
    fn eval_inline(self) -> i64 {
        let mut results: HashMap<usize, i64> = HashMap::with_capacity(self.elems.len());

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            // each node is only referenced once so just remove it
            let node = node.map(|x| results.remove(&x).expect("node not in result map"));
            let alg_res = match node {
                Expr::Add { a, b } => a + b,
                Expr::Sub { a, b } => a - b,
                Expr::Mul { a, b } => a * b,
                Expr::LiteralInt { literal } => literal,
            };
            results.insert(idx, alg_res);
        }

        results.remove(&0).unwrap()
    }
}

// here we fold up the expression tree from the leaves to the root, evaluating it one layer at a time and storing the results in a hashmap until they are used.
// Everything is stored in a vec, so we don't need to worry about the overhead of traversing a bunch of pointers

// ok, still not elegant. Once again, the logic of _how_ we fold layers of recursive structure (Expr<i64>) into a single value (i64)
// is interleaved with a bunch of boilerplate that handles the actual mechanics of recursion.


// RECURSION SCHEMES

// The key idea here, which is taken entirely from recursion schemes, is to _separate_ the mechanism of recursion from the logic of recursion.
// Let's see what that looks like:

impl RecursiveExpr {
    fn fold<A, F: FnMut(Expr<A>) -> A>(self, mut alg: F) -> A {
        let mut results: HashMap<usize, A> = HashMap::with_capacity(self.elems.len());

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            // each node is only referenced once so just remove it
            let node = node.map(|x| results.remove(&x).expect("node not in result map"));
            let alg_res = alg(node);
            results.insert(idx, alg_res);
        }

        results.remove(&0).unwrap()
    }
}


// First, we have a generic representation of folding some structure into a single value - instead of folding an Expr<i64> into a single i64, 
// we fold some Expr<A> into an A. The code looks pretty much the same as eval_inline, but it lets us factor out the mechanism of recursion.

impl RecursiveExpr {
    pub fn eval(self) -> i64 {
        self.fold(|expr| match expr {
            Expr::Add { a, b } => a + b,
            Expr::Sub { a, b } => a - b,
            Expr::Mul { a, b } => a * b,
            Expr::LiteralInt { literal } => literal,
        })
    }
}

// Having done so, we can then write a really clean elegant implementation of eval by folding with pretty much the same logic as we had in eval_inline
// This function is clean and doesn't contain any boilerplate, which means there's less room for bugs. In fact, it actually contains slightly less boilerplate than
// the eval function we wrote for ExprBoxed::eval because it doesn't have to handle recursively calling itself. Even better, it's extremely performant -
// instead of following a bunch of pointers it just calls into_iter on a vector and consumes the resulting iterator (TODO: numbers here)

impl RecursiveExpr {
    fn unfold<A, F: Fn(A) -> Expr<A>>(a: A, coalg: F) -> Self {
        let mut frontier = VecDeque::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop_front() {
            let node = coalg(seed);

            let node = node.map(|aa| {
                frontier.push_back(aa);
                // idx of pointed-to element determined from frontier + elems size
                elems.len() + frontier.len()
            });

            elems.push(node);
        }

        Self { elems }
    }
}

// Second, we have a generic representation of unfolding some structure from a single value - instead of unfolding 
// a single layer of Expr<&ExprBoxed> structure from an &ExprBoxed seed value,
// we unfold some A into an Expr<A>. As with fold, the code looks pretty much the same as from_ast_inline, just with the
// specific unfold logic factored out


impl RecursiveExpr {

    pub fn from_ast(ast: &ExprBoxed) -> Self {
        Self::unfold(ast, |seed| match seed {
            ExprBoxed::Add { a, b } => Expr::Add { a, b },
            ExprBoxed::Sub { a, b } => Expr::Sub { a, b },
            ExprBoxed::Mul { a, b } => Expr::Mul { a, b },
            ExprBoxed::LiteralInt { literal } => Expr::LiteralInt { literal: *literal },
        })
    }
}

// Here's what from_ast looks like as written using unfold. Just as before, there's almost no boilerplate, the body of the function is almost entirely taken
// up by the unfold logic and not the mechanism of recursion.

// I used proptest to test this code, by generating and evaluating many expression trees,
// with each evaluated in parallel by all 3 methods - evaluating a boxed expr directly, evaluating an expr using the inline code, and evaluating it using generic fold/unfold
// Then, I asserted that each method produced the same result.

// Side note:
// This actually helped me find a bug!
// NOTE: this helped me find one serious bug in fold, where it was doing vec pop instead of vec head_pop so switched to VecDequeue. Found minimal example, Add (0, Sub(0, 1)).


// NOTE: not sure where to go from here, but this is blog post one in a series, definitely. maybe end here, with a promise of showing how to do cool async stuff


// generate a bunch of expression trees and evaluate them via each method
#[cfg(test)]
proptest! {
    #[test]
    fn expr_eval(boxed_expr in arb_expr()) {
        let eval_boxed = boxed_expr.eval();

        let eval_inlined = RecursiveExpr::from_ast_inline(&boxed_expr).eval_inline();
        let eval_via_fold = RecursiveExpr::from_ast(&boxed_expr).eval();

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