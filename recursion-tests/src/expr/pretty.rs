use crate::expr::{naive::Expr, ExprFrame};

#[cfg(test)]
use crate::expr::naive::arb_expr;
#[cfg(test)]
use proptest::proptest;

fn prec(expr: &Expr) -> u8 {
    match expr {
        Expr::Add(_, _) | Expr::Sub(_, _) => 1,
        Expr::Mul(_, _) => 2,
        Expr::LiteralInt(_) => 3,
    }
}

/// Pretty-print a expression with minimal parentheses (naive recursive implementation)
pub fn pretty_naive(expr: &Expr) -> String {
    // `is_right` indicates if the current expression is the right-hand side of a non-associative operator
    fn helper(expr: &Expr, parent_prec: u8, is_right_of_sub: bool) -> String {
        let current_prec = prec(expr);

        let mut s = match expr {
            Expr::Add(a, b) => format!(
                "{} + {}",
                helper(a, current_prec, false),
                helper(b, current_prec, false)
            ),
            Expr::Sub(a, b) => format!(
                "{} - {}",
                helper(a, current_prec, false),
                helper(b, current_prec, true)
            ),
            Expr::Mul(a, b) => format!(
                "{} * {}",
                helper(a, current_prec, false),
                helper(b, current_prec, false)
            ),
            Expr::LiteralInt(x) => x.to_string(),
        };

        // Add parentheses if the current operator has lower precedence than the parent,
        // or if it's on the right-hand side of a subtraction and has the same precedence.
        if current_prec < parent_prec || (is_right_of_sub && current_prec == parent_prec) {
            s = format!("({})", s);
        }

        s
    }

    helper(expr, 0, false)
}

/// Pretty-print a expression with minimal parentheses
pub fn pretty_layer(expr: ExprFrame<(&Expr, String)>) -> String {
    match expr {
        ExprFrame::Add((a, mut a_str), (b, mut b_str)) => {
            if prec(a) < 1 {
                a_str = format!("({})", a_str);
            }
            if prec(b) < 1 {
                b_str = format!("({})", b_str);
            }
            format!("{} + {}", a_str, b_str)
        }
        ExprFrame::Sub((a, mut a_str), (b, mut b_str)) => {
            if prec(a) < 1 {
                a_str = format!("({})", a_str);
            }
            if prec(b) <= 1 {
                b_str = format!("({})", b_str);
            }
            format!("{} - {}", a_str, b_str)
        }
        ExprFrame::Mul((a, mut a_str), (b, mut b_str)) => {
            if prec(a) < 2 {
                a_str = format!("({})", a_str);
            }
            if prec(b) < 2 {
                b_str = format!("({})", b_str);
            }
            format!("{} * {}", a_str, b_str)
        }
        ExprFrame::LiteralInt(x) => x.to_string(),
    }
}

#[cfg(test)]
proptest! {
    #[test]
    fn expr_pretty(expr in arb_expr()) {
        use recursion::CollapsibleExt;

        let naive = pretty_naive(&expr);
        let expr = Box::new(expr);
        let pretty = expr.as_ref().collapse_with_structure(pretty_layer);
        assert_eq!(naive, pretty);
    }
}
