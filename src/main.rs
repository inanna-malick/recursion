use std::collections::HashMap;

use exprs::{
    examples::expr::eval::eval,
    examples::expr::naive::{from_ast, ExprAST},
    examples::linked_list::{from_str, to_str},
};

fn main() {
    println!("Hello, world!");

    let test = Box::new(ExprAST::Add(
        Box::new(ExprAST::Mul(
            Box::new(ExprAST::LiteralInt(2)),
            Box::new(ExprAST::LiteralInt(3)),
        )),
        Box::new(ExprAST::LiteralInt(8)),
    ));

    let expr_graph = from_ast(test);

    let evaluated = eval(&HashMap::new(), expr_graph);

    println!("res: {:?}", evaluated);

    let long_string = (0..1000).map(|_| "abc").collect::<String>();

    let long_string_haskell_style = from_str(&long_string);
    let long_string_round_trip = to_str(long_string_haskell_style);

    assert_eq!(long_string, long_string_round_trip);
}
