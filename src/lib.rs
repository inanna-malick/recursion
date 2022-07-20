pub mod functor;
pub mod recursive;
pub mod recursive_block_alloc;
pub mod recursive_dfs;

// using cfg flag to make expr examples available in a benchmark context
#[cfg(any(test, feature = "expr_example"))]
pub mod examples;
