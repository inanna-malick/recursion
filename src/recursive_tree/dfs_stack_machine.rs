use crate::{
    functor::Functor,
    recursive::{Foldable, Generatable},
    recursive_tree::{RecursiveTree, RecursiveTreeRef},
};

// HOLY SHIT: if I build this with a DFS I can use, like, a simple stack to keep track of things
//            like, each eval phase just pops some elements, EXACT OPPOSITE ARROWS OF CONSTRUCTION
// haha nice nice nice nice nice - will just need to change impl here to push and keep it working,
// can impl pop-based situation next. wait, holy shit, if it just runs pop I can have a vec of Expr<()>
// NOTE: adds hard requirement, functor traversal order MUST be constant. woah.
impl<A, U, O: Functor<(), Unwrapped = A, To = U>> Generatable<A, O> for RecursiveTree<U, ()> {
    fn generate_layer<F: Fn(A) -> O>(a: A, coalg: F) -> Self {
        let mut frontier = Vec::from([a]);
        let mut elems = vec![];

        // unfold to build a vec of elems while preserving topo order
        while let Some(seed) = frontier.pop() {
            let node = coalg(seed);

            let mut topush = Vec::new();

            let node = node.fmap(|aa| topush.push(aa));

            frontier.extend(topush.into_iter().rev());

            elems.push(node);
        }

        elems.reverse();

        Self {
            elems,
            _underlying: std::marker::PhantomData,
        }
    }
}

impl<A, O, U: Functor<A, To = O, Unwrapped = ()>> Foldable<A, O> for RecursiveTree<U, ()> {
    fn fold<F: FnMut(O) -> A>(self, mut alg: F) -> A {
        let mut result_stack = Vec::new();

        for node in self.elems.into_iter() {
            let alg_res = {
                // each node is only referenced once so just remove it, also we know it's there so unsafe is fine
                let node = node.fmap(|_| result_stack.pop().unwrap());

                alg(node)
            };
            result_stack.push(alg_res);
        }

        result_stack.pop().unwrap()
    }
}

impl<'a, A, O: 'a, U> Foldable<A, O> for RecursiveTreeRef<'a, U, ()>
where
    &'a U: Functor<A, To = O, Unwrapped = ()>,
{
    fn fold<F: FnMut(O) -> A>(self, mut alg: F) -> A {
        let mut result_stack = Vec::with_capacity(32);

        for node in self.elems.iter() {
            let node = node.fmap(|_| unsafe { result_stack.pop().unwrap_unchecked() });

            result_stack.push(alg(node));
        }

        unsafe { result_stack.pop().unwrap_unchecked() }
    }
}
