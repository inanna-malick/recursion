use crate::functor::Functor;
use crate::stack_machine_lazy::unfold_and_fold_annotate_result;
use futures::future::BoxFuture;
use futures::FutureExt;
use tokio::try_join;

#[derive(Debug, Clone, Copy)]
pub struct DBKey(usize);

// tiny expr thing, just barely enough to have i64's and bool's
#[derive(Debug, Clone)]
pub enum ExprBoxed {
    Add(Box<ExprBoxed>, Box<ExprBoxed>),
    Sub(Box<ExprBoxed>, Box<ExprBoxed>),

    Eq(Box<ExprBoxed>, Box<ExprBoxed>),

    And(Box<ExprBoxed>, Box<ExprBoxed>),

    If(Box<ExprBoxed>, Box<ExprBoxed>, Box<ExprBoxed>),

    LiteralInt(i64),
    LiteralBool(bool),

    DatabaseInt(DBKey), // stub for actual async operation
}

#[derive(Debug, Clone)]
pub enum Expr<A> {
    Add(A, A),
    Sub(A, A),
    Eq(A, A),
    And(A, A),
    If(A, A, A),
    LiteralInt(i64),
    LiteralBool(bool),
    DatabaseInt(DBKey), // stub for actual async operation
}

impl<Out, E> Expr<BoxFuture<'static, Result<Out, E>>> {
    async fn try_join(self) -> Result<Expr<Out>, E> {
        use Expr::*;
        Ok(match self {
            Add(a, b) => {
                let (a, b) = try_join!(a, b)?;
                Add(a, b)
            }
            Sub(a, b) => {
                let (a, b) = try_join!(a, b)?;
                Sub(a, b)
            }
            Eq(a, b) => {
                let (a, b) = try_join!(a, b)?;
                Eq(a, b)
            }
            And(a, b) => {
                let (a, b) = try_join!(a, b)?;
                And(a, b)
            }
            If(a, b, c) => {
                let (a, b, c) = try_join!(a, b, c)?;
                If(a, b, c)
            }
            LiteralInt(x) => LiteralInt(x),
            LiteralBool(x) => LiteralBool(x),
            DatabaseInt(x) => DatabaseInt(x),
        })
    }
}

impl<A, B> Functor<B> for Expr<A> {
    type To = Expr<B>;
    type Unwrapped = A;

    #[inline(always)]
    fn fmap<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            Expr::Add(a, b) => Expr::Add(f(a), f(b)),
            Expr::Sub(a, b) => Expr::Sub(f(a), f(b)),
            Expr::And(a, b) => Expr::And(f(a), f(b)),
            Expr::Eq(a, b) => Expr::Eq(f(a), f(b)),
            Expr::If(a, b, c) => Expr::If(f(a), f(b), f(c)),
            Expr::LiteralInt(i) => Expr::LiteralInt(i),
            Expr::LiteralBool(b) => Expr::LiteralBool(b),
            Expr::DatabaseInt(x) => Expr::DatabaseInt(x),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprRes {
    Bool(bool),
    Int(i64),
}

#[derive(Debug, Clone)]
pub enum ExprType {
    Bool,
    Int,
}

pub fn expand_layer(x: &ExprBoxed) -> Expr<&ExprBoxed> {
    println!("expand: {:?}", x);
    match x {
        ExprBoxed::Add(a, b) => Expr::Add(a, b),
        ExprBoxed::Sub(a, b) => Expr::Sub(a, b),
        ExprBoxed::Eq(a, b) => Expr::Eq(a, b),
        ExprBoxed::And(a, b) => Expr::And(a, b),
        ExprBoxed::LiteralInt(x) => Expr::LiteralInt(*x),
        ExprBoxed::LiteralBool(x) => Expr::LiteralBool(*x),
        ExprBoxed::DatabaseInt(x) => Expr::DatabaseInt(*x),
        ExprBoxed::If(a, b, c) => Expr::If(a, b, c),
    }
}

pub fn typecheck(expr: Expr<ExprType>) -> Result<ExprType, String> {
    use Expr::*;
    use ExprType::*;
    println!("typecheck: {:?}", expr);
    match expr {
        Add(Int, Int) => Ok(Int),
        Add(a, b) => Err(format!("add {:?} {:?}", a, b)),

        Sub(Int, Int) => Ok(Int),
        Sub(a, b) => Err(format!("sub {:?} {:?}", a, b)),

        And(Bool, Bool) => Ok(Bool),
        And(a, b) => Err(format!("and {:?} {:?}", a, b)),

        Eq(Bool, Bool) => Ok(Bool),
        Eq(Int, Int) => Ok(Bool),
        Eq(a, b) => Err(format!("eq {:?} {:?}", a, b)),

        If(Bool, Int, Int) => Ok(Int),
        If(Bool, Bool, Bool) => Ok(Bool),
        If(a, b, c) => Err(format!("if {:?} then {:?} else {:?}", a, b, c)),

        LiteralInt(_) => Ok(Int),

        LiteralBool(_) => Ok(Bool),

        DatabaseInt(_) => Ok(Int),
    }
}

pub fn eval_type_annotated_layer(
    expr_type: ExprType,
    expr: Expr<BoxFuture<'static, Result<ExprRes, String>>>,
) -> BoxFuture<'static, Result<ExprRes, String>> {
    async move {
        use Expr::*;
        use ExprRes::*;


        let expr = expr.try_join().await?;

        println!("eval typed: {:?}", expr);
        match expr {
            Add(Int(a), Int(b)) => Ok(Int(a + b)),
            Sub(Int(a), Int(b)) => Ok(Int(a - b)),

            Eq(Bool(a), Bool(b)) => Ok(Bool(a == b)),
            Eq(Int(a), Int(b)) => Ok(Bool(a == b)),

            And(Bool(a), Bool(b)) => Ok(Bool(a == b)),

            If(Bool(cond), a, b) => Ok(
                if cond {
                    a
                } else {
                    b
                }),

            LiteralInt(i) => Ok(Int(i)),
            LiteralBool(b) => Ok(Bool(b)),

            DatabaseInt(key) => {
                async {
                    if key.0 == 999 {
                        // fail on randomly chosen number to simulate timeout or w/e
                        Err("fail on magic key 999".to_string())
                    } else {
                    // just squint and pretend this is a real async call
                    Ok(Int(1337) )
                    }
                }.await
            }


            invalid => Err(format!(
                "invalid expr eval somehow passed typecheck, programmer error:\n\texpected type: {:?}\n\texpr: {:?}",
                expr_type, invalid
            )),
        }
    }.boxed()
}

pub fn typecheck_and_eval(
    expr: &ExprBoxed,
) -> Result<BoxFuture<'static, Result<ExprRes, String>>, String> {
    unfold_and_fold_annotate_result(
        expr,
        |l| Ok(expand_layer(l)),
        typecheck,
        // only builds a future, doesn't actually run it, so no failure is possible
        |annotation, layer| Ok(eval_type_annotated_layer(annotation, layer)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ExprBoxed::*;

    #[test]
    fn test_if_with_i64_condition() {
        // invalid, can't have i64 condition for "if INT then FOO BAR"
        // if 1 +2 ( 1 - 7) else (-11 - 7)
        let tree = If(
            Box::new(And(Box::new(LiteralInt(1)), Box::new(LiteralInt(2)))),
            Box::new(Sub(Box::new(LiteralInt(1)), Box::new(LiteralInt(7)))),
            Box::new(Sub(Box::new(LiteralInt(-11)), Box::new(LiteralInt(7)))),
        );

        let result = typecheck_and_eval(&tree).err();
        assert_eq!(result, Some("and Int Int".to_string()));
    }

    #[test]
    fn test_if_with_non_matching_branch_types() {
        // invalid, type of if branches must be the same
        // if true then (1 - 7) else false
        let tree = If(
            Box::new(LiteralBool(true)),
            Box::new(Sub(Box::new(LiteralInt(1)), Box::new(LiteralInt(7)))),
            Box::new(LiteralBool(false)),
        );

        let result = typecheck_and_eval(&tree).err();
        assert_eq!(result, Some("if Bool then Int else Bool".to_string()));
    }

    #[test]
    fn test_if_with_broken_eq_test_traversal_ordering() {
        // invalid, type of if branches must be the same AND condition must be of type Bool
        // note that 2 different type errors exist, this shows ordering
        // (back to front in this case, branches eval'd before condition)
        //
        // if (1 == false) then (false - 7) else 3
        let tree = If(
            Box::new(Eq(Box::new(LiteralInt(1)), Box::new(LiteralBool(false)))),
            Box::new(Sub(Box::new(LiteralBool(false)), Box::new(LiteralInt(7)))),
            Box::new(LiteralInt(3)),
        );

        let result = typecheck_and_eval(&tree).err();
        assert_eq!(result, Some("sub Bool Int".to_string()));
    }

    #[test]
    fn test_eq_i64_bool() {
        // invalid, INT == BOOL
        // if 1 == true
        let tree = Eq(Box::new(LiteralInt(1)), Box::new(LiteralBool(true)));

        let result = typecheck_and_eval(&tree).err();
        assert_eq!(result, Some("eq Int Bool".to_string()));
    }

    #[tokio::test]
    async fn test_eval_typed() {
        // valid
        // if false then ( 1 - 7) else (database(1337) - 7)
        let tree = If(
            Box::new(LiteralBool(false)),
            Box::new(Sub(Box::new(LiteralInt(1)), Box::new(LiteralInt(7)))),
            Box::new(Sub(
                Box::new(DatabaseInt(DBKey(
                    1234, /* specific key doesn't matter lol */
                ))),
                Box::new(LiteralInt(7)),
            )),
        );

        let result = typecheck_and_eval(&tree).expect("no type error").await;
        assert_eq!(result, Ok(ExprRes::Int(1330)));
    }

    #[tokio::test]
    async fn test_eval_typed_2() {
        // valid
        // if 1 == 2 ( 1 - 7) else (-11 - 7)
        let tree = If(
            Box::new(Eq(Box::new(LiteralInt(3)), Box::new(LiteralInt(3)))),
            Box::new(Sub(Box::new(LiteralInt(1)), Box::new(LiteralInt(7)))),
            Box::new(Sub(Box::new(LiteralInt(-11)), Box::new(LiteralInt(7)))),
        );

        let result = typecheck_and_eval(&tree).expect("no type error").await;
        assert_eq!(result, Ok(ExprRes::Int(-6)));
    }

    #[tokio::test]
    async fn test_fail_on_db_call() {
        // valid
        // if 1 == 2 ( 1 - 7) else (-11 - database(999 invalid key))
        let tree = If(
            Box::new(Eq(Box::new(LiteralInt(3)), Box::new(LiteralInt(3)))),
            Box::new(Sub(Box::new(LiteralInt(1)), Box::new(LiteralInt(7)))),
            Box::new(Sub(
                Box::new(LiteralInt(-11)),
                Box::new(DatabaseInt(DBKey(999))),
            )),
        );

        let result = typecheck_and_eval(&tree).expect("no type error").await;
        assert_eq!(result, Err("fail on magic key 999".to_string()));
    }
}
