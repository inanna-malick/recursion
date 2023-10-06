/*!

This crate provides tools for working with recursive data structures and computation in
a concise, stack safe, and performant manner.

TODO: discuss separation of logic and machinery of recursion

# Here's how it works: Expr

Let's say you have a recursive data structure - an expression in a simple expression language
that supports a few mathematical operations.

```rust
pub enum Expr {
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    LiteralInt(i64),
}
```

For working with this `Expr` type we'll define a _frame_ type `ExprFrame<A>`.
It's exactly the same as Expr, except `Box<Self>` is replaced with `A`

```rust
pub enum ExprFrame<A> {
    Add(A, A),
    Sub(A, A),
    Mul(A, A),
    LiteralInt(i64),
}
```

Now all we need is some mechanical boilerplate: [`MappableFrame`] for `ExprFrame` and [`Expandable`] and [`Collapsible`] for `Expr`.
I'll elide that for now, but read the documentation for the above traits to learn what they do and how to implement them.

# Collapsing an Expr into a value

We'll be working with this `Expr` (constructed via some elided helper functions)
```rust
# pub enum Expr {
#     Add(Box<Expr>, Box<Expr>),
#     Sub(Box<Expr>, Box<Expr>),
#     Mul(Box<Expr>, Box<Expr>),
#     LiteralInt(i64),
# }
#     fn add(a: Expr, b: Expr) -> Expr {
#         Expr::Add(Box::new(a), Box::new(b))
#     }
#     fn subtract(a: Expr, b: Expr) -> Expr {
#         Expr::Sub(Box::new(a), Box::new(b))
#     }
#     fn multiply(a: Expr, b: Expr) -> Expr {
#         Expr::Mul(Box::new(a), Box::new(b))
#     }
#     fn literal(n: i64) -> Expr {
#         Expr::LiteralInt(n)
#     }
// (1 - 2) * 3
let expr = multiply(subtract(literal(1), literal(2)), literal(3));
```

This example shows how to evaluate expression using this idiom:

```rust
# pub enum Expr {
#     Add(Box<Expr>, Box<Expr>),
#     Sub(Box<Expr>, Box<Expr>),
#     Mul(Box<Expr>, Box<Expr>),
#     LiteralInt(i64),
# }
#     fn add(a: Expr, b: Expr) -> Expr {
#         Expr::Add(Box::new(a), Box::new(b))
#     }
#     fn subtract(a: Expr, b: Expr) -> Expr {
#         Expr::Sub(Box::new(a), Box::new(b))
#     }
#     fn multiply(a: Expr, b: Expr) -> Expr {
#         Expr::Mul(Box::new(a), Box::new(b))
#     }
#     fn literal(n: i64) -> Expr {
#         Expr::LiteralInt(n)
#     }
# pub enum ExprFrame<A> {
#     Add(A, A),
#     Sub(A, A),
#     Mul(A, A),
#     LiteralInt(i64),
# }
# use recursion::*;
# impl MappableFrame for ExprFrame<PartiallyApplied> {
#     type Frame<X> = ExprFrame<X>;
#     fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
#         match input {
#             ExprFrame::Add(a, b) => ExprFrame::Add(f(a), f(b)),
#             ExprFrame::Sub(a, b) => ExprFrame::Sub(f(a), f(b)),
#             ExprFrame::Mul(a, b) => ExprFrame::Mul(f(a), f(b)),
#             ExprFrame::LiteralInt(x) => ExprFrame::LiteralInt(x),
#         }
#     }
# }
# impl<'a> Collapsible for &'a Expr {
#     type FrameToken = ExprFrame<PartiallyApplied>;
#     fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
#         match self {
#             Expr::Add(a, b) => ExprFrame::Add(a, b),
#             Expr::Sub(a, b) => ExprFrame::Sub(a, b),
#             Expr::Mul(a, b) => ExprFrame::Mul(a, b),
#             Expr::LiteralInt(x) => ExprFrame::LiteralInt(*x),
#         }
#     }
# }
# let expr = multiply(subtract(literal(1), literal(2)), literal(3));
fn eval(e: &Expr) -> i64 {
    e.collapse_frames(|frame| match frame {
        ExprFrame::Add(a, b) => a + b,
        ExprFrame::Sub(a, b) => a - b,
        ExprFrame::Mul(a, b) => a * b,
        ExprFrame::LiteralInt(x) => x,
    })
}

assert_eq!(eval(&expr), -3);
```

# Fallible functions

At this point, you may have noticed something: I've ommited division, which is a fallible operation
because division by 0 is undefined. Here's how to implement a division function that returns an `Err`
if the expression attempts division by 0:

```rust
# pub enum Expr {
#     Add(Box<Expr>, Box<Expr>),
#     Sub(Box<Expr>, Box<Expr>),
#     Mul(Box<Expr>, Box<Expr>),
#     Div(Box<Expr>, Box<Expr>),
#     LiteralInt(i64),
# }
#     fn add(a: Expr, b: Expr) -> Expr {
#         Expr::Add(Box::new(a), Box::new(b))
#     }
#     fn subtract(a: Expr, b: Expr) -> Expr {
#         Expr::Sub(Box::new(a), Box::new(b))
#     }
#     fn multiply(a: Expr, b: Expr) -> Expr {
#         Expr::Mul(Box::new(a), Box::new(b))
#     }
#     fn divide(a: Expr, b: Expr) -> Expr {
#         Expr::Div(Box::new(a), Box::new(b))
#     }
#     fn literal(n: i64) -> Expr {
#         Expr::LiteralInt(n)
#     }
# pub enum ExprFrame<A> {
#     Add(A, A),
#     Sub(A, A),
#     Mul(A, A),
#     Div(A, A),
#     LiteralInt(i64),
# }
# use recursion::*;
# impl MappableFrame for ExprFrame<PartiallyApplied> {
#     type Frame<X> = ExprFrame<X>;
#     fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
#         match input {
#             ExprFrame::Add(a, b) => ExprFrame::Add(f(a), f(b)),
#             ExprFrame::Sub(a, b) => ExprFrame::Sub(f(a), f(b)),
#             ExprFrame::Mul(a, b) => ExprFrame::Mul(f(a), f(b)),
#             ExprFrame::Div(a, b) => ExprFrame::Div(f(a), f(b)),
#             ExprFrame::LiteralInt(x) => ExprFrame::LiteralInt(x),
#         }
#     }
# }
# impl<'a> Collapsible for &'a Expr {
#     type FrameToken = ExprFrame<PartiallyApplied>;
#     fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
#         match self {
#             Expr::Add(a, b) => ExprFrame::Add(a, b),
#             Expr::Sub(a, b) => ExprFrame::Sub(a, b),
#             Expr::Mul(a, b) => ExprFrame::Mul(a, b),
#             Expr::Div(a, b) => ExprFrame::Div(a, b),
#             Expr::LiteralInt(x) => ExprFrame::LiteralInt(*x),
#         }
#     }
# }

fn try_eval(e: &Expr) -> Result<i64, &str> {
    e.try_collapse_frames(|frame| match frame {
                ExprFrame::Add(a, b) => Ok(a + b),
                ExprFrame::Sub(a, b) => Ok(a - b),
                ExprFrame::Mul(a, b) => Ok(a * b),
                ExprFrame::Div(a, b) =>
                    if b == 0 { Err("cannot divide by zero")} else {Ok(a / b)},
                ExprFrame::LiteralInt(x) => Ok(x),
    })
}

let valid_expr = multiply(subtract(literal(1), literal(2)), literal(3));
let invalid_expr = divide(literal(2), literal(0));

assert_eq!(try_eval(&valid_expr), Ok(-3));
assert_eq!(try_eval(&invalid_expr), Err("cannot divide by zero"));
```

# Expanding an Expr from a seed value

Here's an example showing how to expand an `Expr` from a seed value

```rust
# #[derive(Debug, PartialEq, Eq)]
# pub enum Expr {
#     Add(Box<Expr>, Box<Expr>),
#     Sub(Box<Expr>, Box<Expr>),
#     Mul(Box<Expr>, Box<Expr>),
#     LiteralInt(i64),
# }
#     fn add(a: Expr, b: Expr) -> Expr {
#         Expr::Add(Box::new(a), Box::new(b))
#     }
#     fn subtract(a: Expr, b: Expr) -> Expr {
#         Expr::Sub(Box::new(a), Box::new(b))
#     }
#     fn multiply(a: Expr, b: Expr) -> Expr {
#         Expr::Mul(Box::new(a), Box::new(b))
#     }
#     fn literal(n: i64) -> Expr {
#         Expr::LiteralInt(n)
#     }
# pub enum ExprFrame<A> {
#     Add(A, A),
#     Sub(A, A),
#     Mul(A, A),
#     LiteralInt(i64),
# }
# use recursion::*;
# impl MappableFrame for ExprFrame<PartiallyApplied> {
#     type Frame<X> = ExprFrame<X>;
#     fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
#         match input {
#             ExprFrame::Add(a, b) => ExprFrame::Add(f(a), f(b)),
#             ExprFrame::Sub(a, b) => ExprFrame::Sub(f(a), f(b)),
#             ExprFrame::Mul(a, b) => ExprFrame::Mul(f(a), f(b)),
#             ExprFrame::LiteralInt(x) => ExprFrame::LiteralInt(x),
#         }
#     }
# }
# impl<'a> Collapsible for &'a Expr {
#     type FrameToken = ExprFrame<PartiallyApplied>;
#     fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
#         match self {
#             Expr::Add(a, b) => ExprFrame::Add(a, b),
#             Expr::Sub(a, b) => ExprFrame::Sub(a, b),
#             Expr::Mul(a, b) => ExprFrame::Mul(a, b),
#             Expr::LiteralInt(x) => ExprFrame::LiteralInt(*x),
#         }
#     }
# }
# impl Expandable for Expr {
#     type FrameToken = ExprFrame<PartiallyApplied>;
#     fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self {
#         match val {
#             ExprFrame::Add(a, b) => Expr::Add(Box::new(a), Box::new(b)),
#             ExprFrame::Sub(a, b) => Expr::Sub(Box::new(a), Box::new(b)),
#             ExprFrame::Mul(a, b) => Expr::Mul(Box::new(a), Box::new(b)),
#             ExprFrame::LiteralInt(x) => Expr::LiteralInt(x),
#         }
#     }
# }
fn build_expr(depth: usize) -> Expr {
    Expr::expand_frames(depth, |depth| {
        if depth > 0 {
            ExprFrame::Add(depth - 1, depth - 1)
        } else {
            ExprFrame::LiteralInt(1)
        }
    })
}

let expected = add(add(literal(1), literal(1)), add(literal(1), literal(1)));

assert_eq!(expected, build_expr(2));

```


# Misc etc

TODO: It borrows some tricks from Haskell - specifically recursion schemes - discuss this

TODO: generate visualizations for the above and include them


 */
mod frame;
mod recursive;

#[cfg(feature = "experimental")]
pub mod experimental;

pub use frame::{MappableFrame, PartiallyApplied};
pub use recursive::{Collapsible, CollapsibleExt, Expandable, ExpandableExt};
