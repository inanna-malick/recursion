extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use quote::*;
use proc_macro::TokenStream;

#[proc_macro_derive(HelloWorld)]
pub fn hello_world(input: TokenStream) -> TokenStream {
    // Construct a string representation of the type definition
    let s = input.to_string();
    
    // Parse the string representation
    let ast = syn::parse_derive_input(&s).unwrap();

    // Build the impl
    impl_hello_world(&ast)
}

fn impl_hello_world(ast: &syn::DeriveInput) -> TokenStream {
    let base_type = &ast.ident;

    quote! {
        pub struct RecursiveExpr {
            // nonempty, in topological-sorted order
            elems: Vec<Expr<usize>>,
        }

        impl RecursiveExpr {
            pub fn ana<A, F: Fn(A) -> Expr<A>>(a: A, coalg: F) -> Self {
                let mut frontier = VecDeque::from([a]);
                let mut elems = vec![];

                // unfold to build a vec of elems while preserving topo order
                while let Some(seed) = frontier.pop_front() {
                    let node = coalg(seed);

                    let node: Expr<usize> = node.fmap_into(|aa| {
                        frontier.push_back(aa);
                        // this is the sketchy bit, here - idx of pointed-to element
                        elems.len() + frontier.len()
                    });

                    elems.push(node);
                }

                Self { elems }
            }

            pub fn cata<A, F: FnMut(Expr<A>) -> A>(self, mut alg: F) -> A {
                let mut results: HashMap<usize, A> = HashMap::with_capacity(self.elems.len());

                for (idx, node) in self.elems.into_iter().enumerate().rev() {
                    let alg_res = {
                        // each node is only referenced once so just remove it to avoid cloning owned data
                        let node = node.fmap_into(|x| results.remove(&x).expect("node not in result map"));
                        alg(node)
                    };
                    results.insert(idx, alg_res);
                }

                // assumes nonempty recursive structure
                results.remove(&0).unwrap()
            }

            // HAHA HOLY SHIT THIS RULES IT WORKS IT WORKS IT WORKS, GET A POSTGRES TEST GOING BECAUSE THIS RULES
            pub async fn cata_async<
                'a,
                A: Send + Sync + 'a,
                E: Send + 'a,
                F: Fn(Expr<A>) -> BoxFuture<'a, Result<A, E>> + Send + Sync + 'a,
            >(
                self,
                alg: F,
            ) -> Result<A, E> {
                let execution_graph = self.cata(|e|
                    // NOTE: want to directly pass in fn but can't because borrow checker - not sure how to do this, causes spurious clippy warning
                    cata_async_helper(e,  |x| alg(x)));

                execution_graph.await
            }
        }

        // given an async fun, build an execution graph from cata async
        fn cata_async_helper<
            'a,
            A: Send + 'a,
            E: 'a,
            F: Fn(Expr<A>) -> BoxFuture<'a, Result<A, E>> + Send + Sync + 'a,
        >(
            e: Expr<BoxFuture<'a, Result<A, E>>>,
            f: F,
        ) -> BoxFuture<'a, Result<A, E>> {
            async move {
                let e = e.try_join_expr().await?;
                f(e).await
            }
            .boxed()
        }
    }
}
