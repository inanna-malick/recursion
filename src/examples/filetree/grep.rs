use crate::examples::filetree::{FileTree, RecursiveFileTree};
use crate::recursive::{Recursive};
use futures::{future::BoxFuture, FutureExt};
use std::{
    fs::Metadata,
    path::{PathBuf},
};


#[derive(Debug, Clone)]
pub struct GrepResult {
    pub path: PathBuf,
    pub metadata: Metadata,
    pub contents: String,
}

impl RecursiveFileTree {
    // return vec of grep results, with short circuit
    pub fn grep<'a, F: for<'x> Fn(&'x PathBuf) -> bool + Send + Sync + 'a>(
        self,
        root_dir: PathBuf,
        substring: &'a str,
        filter: &'a F,
    ) -> BoxFuture<'a, std::io::Result<Vec<GrepResult>>> {
        println!("grep");
        let f = self.cata(move |node| {
            Box::new(move |path| {
                async move { alg::<'a, _>(node, path, substring, filter).await }.boxed()
            })
        });

        f(root_dir)
    }
}

// lazy traversal of filetree with path component
type LazilyTraversableFileTree<'a, Res, Err> =
    FileTree<Box<dyn FnOnce(PathBuf) -> BoxFuture<'a, Result<Res, Err>> + Send + Sync + 'a>>;

// grep a single layer of recursive FileTree structure
async fn alg<'a, F: for<'x> Fn(&'x PathBuf) -> bool + Send + Sync + 'a>(
    node: LazilyTraversableFileTree<'a, Vec<GrepResult>, std::io::Error>,
    path: PathBuf,
    substring: &'a str,
    filter: &'a F,
) -> std::io::Result<Vec<GrepResult>> {
    match node {
        FileTree::File(metadata) => {
            let contents = tokio::fs::read_to_string(&path).await?;
            Ok(if contents.contains(substring) {
                vec![GrepResult {
                    path,
                    metadata,
                    contents,
                }]
            } else {
                Vec::new()
            })
        }
        FileTree::Dir(search_results_futs) => {
            let mut all_results = Vec::new();
            for (path_component, search_result_fut) in search_results_futs.into_iter() {
                let mut child_path = path.clone();
                child_path.push(path_component);
                if filter(&child_path) {
                    // only expand child search branch if filter fn returns true
                    let search_result = search_result_fut(child_path).await?;
                    all_results.extend(search_result.into_iter());
                }
            }
            Ok(all_results)
        }
    }
}
