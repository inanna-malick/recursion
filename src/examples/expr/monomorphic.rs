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

/// simple naive representation of a recursive expression AST. (todo: find better word, 'naive' sounds like shit)
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

pub fn naive_parse(expr: &serde_json::Value) -> ExprBoxed {
    let obj = expr.as_object().expect("can only parse objects");
    let type_tag = obj.get("type").expect("type tag not present");
    let type_tag = type_tag.as_str().expect("non-string type tagr");
    match type_tag {
        "add" => {
            let a = Box::new(naive_parse(obj.get("a").expect("a not present")));
            let b = Box::new(naive_parse(obj.get("b").expect("b not present")));
            ExprBoxed::Add { a, b }
        }
        "subtract" => {
            let a = Box::new(naive_parse(obj.get("a").expect("a not present")));
            let b = Box::new(naive_parse(obj.get("b").expect("b not present")));
            ExprBoxed::Sub { a, b }
        }
        "multiply" => {
            let a = Box::new(naive_parse(obj.get("a").expect("a not present")));
            let b = Box::new(naive_parse(obj.get("b").expect("b not present")));
            ExprBoxed::Mul { a, b }
        }
        "literal" => {
            let literal = obj
                .get("literal")
                .expect("literal not present")
                .as_i64()
                .expect("literal not an i64");
            ExprBoxed::LiteralInt { literal }
        }
        _ => panic!("invalid type tag"),
    }
}

pub fn naive_eval(expr: ExprBoxed) -> i64 {
    match expr {
        ExprBoxed::Add { a, b } => naive_eval(*a) + naive_eval(*b),
        ExprBoxed::Sub { a, b } => naive_eval(*a) - naive_eval(*b),
        ExprBoxed::Mul { a, b } => naive_eval(*a) * naive_eval(*b),
        ExprBoxed::LiteralInt { literal } => literal,
    }
}

/// Simple expression language with some operations on integers
#[derive(Debug, Clone, Copy)]
pub enum Expr<A> {
    Add { a: A, b: A },
    Sub { a: A, b: A },
    Mul { a: A, b: A },
    LiteralInt { literal: i64 },
}

pub struct RecursiveExpr {
    // nonempty, in topological-sorted order
    elems: Vec<Expr<usize>>,
}

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

impl RecursiveExpr {
    pub fn eval(self) -> i64 {
        self.fold( |expr| match expr {
            Expr::Add { a, b } => a + b,
            Expr::Sub { a, b } => a - b,
            Expr::Mul { a, b } => a * b,
            Expr::LiteralInt { literal } => literal,
        })
    }

    pub fn from_ast(ast: Box<ExprBoxed>) -> Self {
        Self::unfold(ast, |x| match *x {
            ExprBoxed::Add { a, b } => Expr::Add { a, b },
            ExprBoxed::Sub { a, b } => Expr::Sub { a, b },
            ExprBoxed::Mul { a, b } => Expr::Mul { a, b },
            ExprBoxed::LiteralInt { literal } => Expr::LiteralInt { literal },
        })
    }

    pub fn parse_json(expr: &serde_json::Value) -> Self {
        Self::unfold(expr, |expr| {
            let obj = expr.as_object().expect("can only parse objects");
            let type_tag = obj.get("type").expect("type tag not present");
            let type_tag = type_tag.as_str().expect("non-string type tagr");
            match type_tag {
                "add" => {
                    let a = obj.get("a").expect("a not present");
                    let b = obj.get("b").expect("b not present");
                    Expr::Add { a, b }
                }
                "subtract" => {
                    let a = obj.get("a").expect("a not present");
                    let b = obj.get("b").expect("b not present");
                    Expr::Sub { a, b }
                }
                "multiply" => {
                    let a = obj.get("a").expect("a not present");
                    let b = obj.get("b").expect("b not present");
                    Expr::Mul { a, b }
                }
                "literal" => {
                    let literal = obj
                        .get("literal")
                        .expect("literal not present")
                        .as_i64()
                        .expect("literal not an i64");
                    Expr::LiteralInt { literal }
                }
                _ => panic!("invalid type tag"),
            }
        })
    }
}

// TODO: talk to plaidfinch about this
impl RecursiveExpr {
    fn fold<A, F: FnMut(Expr<A>) -> A>(self, mut alg: F) -> A {
        let mut results: HashMap<usize, A> = HashMap::with_capacity(self.elems.len());

        for (idx, node) in self.elems.into_iter().enumerate().rev() {
            let alg_res = {
                // each node is only referenced once so just remove it
                let node = node.map(|x| results.remove(&x).expect("node not in result map"));
                alg(node)
            };
            results.insert(idx, alg_res);
        }

        results.remove(&0).unwrap()
    }

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
