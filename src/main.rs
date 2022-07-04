use std::collections::HashMap;

use petgraph_schemes::*;

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

    // LMAO! Ok now it's time to figure out how to render an AST for debugging purposes
    // NOTE: wait, what the fuck, this actually works? spooky, do some extensive testing

    // wait! I can do property based testing (also leave comments in as blog post notes)
    // proptest strat (via rain):
    // have simple stack overflow prone impl that is obviously correct, and also actual impl via from_ast/eval
    // generate many many expression trees and run them through both, asserting that the result is the same

    // this actually provides a viable strategy that doesn't require writing a bunch of box box box etc boilerplate yay
}
