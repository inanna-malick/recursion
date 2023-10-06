mod expr1 {
    use std::fmt::Display;

    use recursion::*;
    use recursion_visualize::visualize::*;
    #[derive(Debug, PartialEq, Eq)]
    pub enum Expr {
        Add(Box<Expr>, Box<Expr>),
        Sub(Box<Expr>, Box<Expr>),
        Mul(Box<Expr>, Box<Expr>),
        LiteralInt(i64),
    }
    impl Display for Expr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Expr::Add(a, b) => write!(f, "{} + {}", a, b),
                Expr::Sub(a, b) => write!(f, "{} - {}", a, b),
                Expr::Mul(a, b) => write!(f, "{} * {}", a, b),
                Expr::LiteralInt(x) => write!(f, "{}", x),
            }
        }
    }

    pub fn add(a: Expr, b: Expr) -> Expr {
        Expr::Add(Box::new(a), Box::new(b))
    }
    pub fn subtract(a: Expr, b: Expr) -> Expr {
        Expr::Sub(Box::new(a), Box::new(b))
    }
    pub fn multiply(a: Expr, b: Expr) -> Expr {
        Expr::Mul(Box::new(a), Box::new(b))
    }
    pub fn literal(n: i64) -> Expr {
        Expr::LiteralInt(n)
    }
    pub enum ExprFrame<A> {
        Add(A, A),
        Sub(A, A),
        Mul(A, A),
        LiteralInt(i64),
    }

    impl Display for ExprFrame<()> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ExprFrame::Add(_, _) => write!(f, "_ + _",),
                ExprFrame::Sub(_, _) => write!(f, "_ - _",),
                ExprFrame::Mul(_, _) => write!(f, "_ * _",),
                ExprFrame::LiteralInt(x) => write!(f, "{}", x),
            }
        }
    }

    impl MappableFrame for ExprFrame<PartiallyApplied> {
        type Frame<X> = ExprFrame<X>;
        fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
            match input {
                ExprFrame::Add(a, b) => ExprFrame::Add(f(a), f(b)),
                ExprFrame::Sub(a, b) => ExprFrame::Sub(f(a), f(b)),
                ExprFrame::Mul(a, b) => ExprFrame::Mul(f(a), f(b)),
                ExprFrame::LiteralInt(x) => ExprFrame::LiteralInt(x),
            }
        }
    }
    impl<'a> Collapsible for &'a Expr {
        type FrameToken = ExprFrame<PartiallyApplied>;
        fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
            match self {
                Expr::Add(a, b) => ExprFrame::Add(a, b),
                Expr::Sub(a, b) => ExprFrame::Sub(a, b),
                Expr::Mul(a, b) => ExprFrame::Mul(a, b),
                Expr::LiteralInt(x) => ExprFrame::LiteralInt(*x),
            }
        }
    }
    impl Expandable for Expr {
        type FrameToken = ExprFrame<PartiallyApplied>;
        fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self {
            match val {
                ExprFrame::Add(a, b) => Expr::Add(Box::new(a), Box::new(b)),
                ExprFrame::Sub(a, b) => Expr::Sub(Box::new(a), Box::new(b)),
                ExprFrame::Mul(a, b) => Expr::Mul(Box::new(a), Box::new(b)),
                ExprFrame::LiteralInt(x) => Expr::LiteralInt(x),
            }
        }
    }

    pub fn eval(e: &Expr) -> (i64, Viz) {
        e.collapse_frames_v(|frame| match frame {
            ExprFrame::Add(a, b) => a + b,
            ExprFrame::Sub(a, b) => a - b,
            ExprFrame::Mul(a, b) => a * b,
            ExprFrame::LiteralInt(x) => x,
        })
    }

    pub fn build_expr(depth: usize) -> (Expr, Viz) {
        Expr::expand_frames_v(depth, |depth| {
            if depth > 0 {
                ExprFrame::Add(depth - 1, depth - 1)
            } else {
                ExprFrame::LiteralInt(1)
            }
        })
    }
}

mod expr2 {
    use std::fmt::Display;

    use recursion::*;
    use recursion_visualize::visualize::{CollapsibleVizExt, Viz};
    pub enum Expr {
        Add(Box<Expr>, Box<Expr>),
        Sub(Box<Expr>, Box<Expr>),
        Mul(Box<Expr>, Box<Expr>),
        Div(Box<Expr>, Box<Expr>),
        LiteralInt(i64),
    }
    impl Display for Expr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Expr::Add(a, b) => write!(f, "{} + {}", a, b),
                Expr::Sub(a, b) => write!(f, "{} - {}", a, b),
                Expr::Mul(a, b) => write!(f, "{} * {}", a, b),
                Expr::Div(a, b) => write!(f, "{} / {}", a, b),
                Expr::LiteralInt(x) => write!(f, "{}", x),
            }
        }
    }
    pub fn add(a: Expr, b: Expr) -> Expr {
        Expr::Add(Box::new(a), Box::new(b))
    }
    pub fn subtract(a: Expr, b: Expr) -> Expr {
        Expr::Sub(Box::new(a), Box::new(b))
    }
    pub fn multiply(a: Expr, b: Expr) -> Expr {
        Expr::Mul(Box::new(a), Box::new(b))
    }
    pub fn divide(a: Expr, b: Expr) -> Expr {
        Expr::Div(Box::new(a), Box::new(b))
    }
    pub fn literal(n: i64) -> Expr {
        Expr::LiteralInt(n)
    }
    pub enum ExprFrame<A> {
        Add(A, A),
        Sub(A, A),
        Mul(A, A),
        Div(A, A),
        LiteralInt(i64),
    }
    impl Display for ExprFrame<()> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ExprFrame::Add(_, _) => write!(f, "_ + _",),
                ExprFrame::Sub(_, _) => write!(f, "_ - _",),
                ExprFrame::Mul(_, _) => write!(f, "_ * _",),
                ExprFrame::Div(_, _) => write!(f, "_ / _",),
                ExprFrame::LiteralInt(x) => write!(f, "{}", x),
            }
        }
    }
    impl MappableFrame for ExprFrame<PartiallyApplied> {
        type Frame<X> = ExprFrame<X>;
        fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
            match input {
                ExprFrame::Add(a, b) => ExprFrame::Add(f(a), f(b)),
                ExprFrame::Sub(a, b) => ExprFrame::Sub(f(a), f(b)),
                ExprFrame::Mul(a, b) => ExprFrame::Mul(f(a), f(b)),
                ExprFrame::Div(a, b) => ExprFrame::Div(f(a), f(b)),
                ExprFrame::LiteralInt(x) => ExprFrame::LiteralInt(x),
            }
        }
    }
    impl<'a> Collapsible for &'a Expr {
        type FrameToken = ExprFrame<PartiallyApplied>;
        fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
            match self {
                Expr::Add(a, b) => ExprFrame::Add(a, b),
                Expr::Sub(a, b) => ExprFrame::Sub(a, b),
                Expr::Mul(a, b) => ExprFrame::Mul(a, b),
                Expr::Div(a, b) => ExprFrame::Div(a, b),
                Expr::LiteralInt(x) => ExprFrame::LiteralInt(*x),
            }
        }
    }

    pub fn try_eval(e: &Expr) -> (Result<i64, &str>, Viz) {
        e.try_collapse_frames_v(|frame| match frame {
            ExprFrame::Add(a, b) => Ok(a + b),
            ExprFrame::Sub(a, b) => Ok(a - b),
            ExprFrame::Mul(a, b) => Ok(a * b),
            ExprFrame::Div(a, b) => {
                if b == 0 {
                    Err("cannot divide by zero")
                } else {
                    Ok(a / b)
                }
            }
            ExprFrame::LiteralInt(x) => Ok(x),
        })
    }
}

fn main() {
    {
        use expr1::*;
        let expr = multiply(subtract(literal(1), literal(2)), literal(3));

        let (evaluated, viz) = eval(&expr);
        assert_eq!(evaluated, -3);

        viz.label("Evaluate Expr".to_string(), "(1 - 2) * 3".to_string())
            .write("eval.html".to_string());

        let (built_expr, viz) = build_expr(2);
        let expected = add(add(literal(1), literal(1)), add(literal(1), literal(1)));

        assert_eq!(built_expr, expected);

        viz.label("Build Expr".to_string(), "1 + 1 + 1 + 1".to_string())
            .write("build_expr.html".to_string());
    }

    {
        use expr2::*;

        let valid_expr = divide(subtract(literal(1), literal(7)), literal(3));
        let invalid_expr = divide(multiply(literal(2), literal(3)), literal(0));

        let (valid_res, valid_viz) = try_eval(&valid_expr);
        let (invalid_res, invalid_viz) = try_eval(&invalid_expr);

        assert_eq!(valid_res, Ok(-2));
        assert_eq!(invalid_res, Err("cannot divide by zero"));

        valid_viz
            .label("Try Eval Valid Expr".to_string(), "(1 - 7) / 3".to_string())
            .write("try_eval_valid.html".to_string());
        invalid_viz
            .label("Try Eval Expr".to_string(), "(2 * 3) / 0".to_string())
            .write("try_eval_invalid.html".to_string());
    }
}
