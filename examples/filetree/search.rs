use crate::filetree::{FileTree, RecursiveFileTree};
use schemes::recursive::Recursive;
use futures::{future::BoxFuture, FutureExt};
use regex::Regex;
use std::{fs::Metadata, path::PathBuf};

pub type LineNumber = usize;

#[derive(Debug, Clone)]
pub struct GrepResult {
    pub path: PathBuf,
    pub metadata: Metadata,
    pub matching_lines: Vec<(LineNumber, String)>,
}

// return vec of grep results, with short circuit
pub fn search<'a>(
    tree: RecursiveFileTree,
    root_dir: PathBuf,
    regex: &'a Regex,
) -> BoxFuture<'a, std::io::Result<Vec<GrepResult>>> {
    let f = tree.fold(move |node| {
        Box::new(move |path| async move { grep_layer(node, path, regex).await }.boxed())
    });

    f(root_dir)
}

// lazy traversal of filetree with path component
type LazilyTraversableFileTree<'a, Res, Err> =
    FileTree<Box<dyn FnOnce(PathBuf) -> BoxFuture<'a, Result<Res, Err>> + Send + Sync + 'a>>;

// grep a single layer of recursive FileTree structure
async fn grep_layer<'a>(
    node: LazilyTraversableFileTree<'a, Vec<GrepResult>, std::io::Error>,
    path: PathBuf,
    regex: &'a Regex,
) -> std::io::Result<Vec<GrepResult>> {
    match node {
        FileTree::File(metadata) => {
            let mut matching_lines = Vec::new();

            match tokio::fs::read_to_string(&path).await {
                Err(_) => {} // binary file or w/e, just skip. TODO: more granular handling
                Ok(contents) => {
                    for (line_num, line) in contents.lines().enumerate() {
                        if regex.is_match(line) {
                            matching_lines.push((line_num, line.to_string()));
                        }
                    }
                }
            }

            Ok(if !matching_lines.is_empty() {
                vec![GrepResult {
                    path,
                    metadata,
                    matching_lines,
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
                let search_result = search_result_fut(child_path).await?;
                all_results.extend(search_result.into_iter());
            }
            Ok(all_results)
        }
    }
}
