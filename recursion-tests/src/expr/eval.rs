use crate::expr::ExprFrame;

#[cfg(test)]
use crate::expr::naive::arb_expr;
use crate::expr::naive::Expr;
use futures::future::BoxFuture;
#[cfg(test)]
use proptest::proptest;

#[derive(Debug, Clone)]
pub struct ValidInt(i64);

#[derive(Debug, Clone)]
pub enum CompiledExpr<A> {
    Add(A, A),
    Sub(A, A),
    Mul(A, A),
    LiteralInt(ValidInt),
}

type CompileError = &'static str;

// only looks at literal case - add/sub/mul ops are always valid
pub fn compile<A>(expr: ExprFrame<A>) -> Result<CompiledExpr<A>, CompileError> {
    match expr {
        ExprFrame::Add(a, b) => Ok(CompiledExpr::Add(a, b)),
        ExprFrame::Sub(a, b) => Ok(CompiledExpr::Sub(a, b)),
        ExprFrame::Mul(a, b) => Ok(CompiledExpr::Mul(a, b)), // TODO: look into futumorphism to return multiple layers here
        ExprFrame::LiteralInt(x) => {
            // arbitrary check
            if x > 99 {
                return Err("invalid literal");
            }

            Ok(CompiledExpr::LiteralInt(ValidInt(x)))
        }
    }
}

pub fn eval_compiled(expr: CompiledExpr<i64>) -> i64 {
    match expr {
        CompiledExpr::Add(a, b) => a + b,
        CompiledExpr::Sub(a, b) => a - b,
        CompiledExpr::Mul(a, b) => a * b,
        CompiledExpr::LiteralInt(ValidInt(x)) => x,
    }
}

#[inline(always)]
pub fn eval_layer(node: ExprFrame<i64>) -> i64 {
    match node {
        ExprFrame::Add(a, b) => a + b,
        ExprFrame::Sub(a, b) => a - b,
        ExprFrame::Mul(a, b) => a * b,
        ExprFrame::LiteralInt(x) => x,
    }
}

pub fn eval_layer_async<'a>(node: ExprFrame<i64>) -> BoxFuture<'a, Result<i64, String>> {
    use futures::FutureExt;
    futures::future::ready(Ok(match node {
        ExprFrame::Add(a, b) => a + b,
        ExprFrame::Sub(a, b) => a - b,
        ExprFrame::Mul(a, b) => a * b,
        ExprFrame::LiteralInt(x) => x,
    }))
    .boxed()
}

pub fn naive_eval(expr: &Expr) -> i64 {
    match expr {
        Expr::Add(a, b) => naive_eval(a) + naive_eval(b),
        Expr::Sub(a, b) => naive_eval(a) - naive_eval(b),
        Expr::Mul(a, b) => naive_eval(a) * naive_eval(b),
        Expr::LiteralInt(x) => *x,
    }
}

// generate a bunch of expression trees and evaluate them
#[cfg(test)]
proptest! {
    #[test]
    fn expr_eval(expr in arb_expr()) {
        use recursion::CollapsibleExt;

        // NOTE: this helped me find one serious bug in the old cata impl,
        //   where it was doing vec pop instead of vec head_pop so switched to VecDequeue.
        //   Found minimal example, Add (0, Sub(0, 1)).
        let simple = naive_eval(&expr);
        let expr = Box::new(expr);
        let eval_gat = expr.as_ref().collapse_frames(eval_layer);
        let eval_gat_try: Result<i64, String> = expr.as_ref().try_collapse_frames(|x| Ok(eval_layer(x)));


        // simple async eval, but really - TODO: something more definitively impressive
        let eval_gat_async = {
            use recursion::experimental::recursive::collapse::CollapsibleAsync;

            let rt = tokio::runtime::Runtime::new().unwrap();

            rt.block_on(async {
                let expr = Box::new(expr.clone());
                expr.collapse_frames_async(eval_layer_async).await
            })
        };

        assert_eq!(simple, eval_gat);
        assert_eq!(Ok(simple), eval_gat_async);
        assert_eq!(Ok(simple), eval_gat_try);
    }

}
