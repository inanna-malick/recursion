use std::{collections::HashMap, default};

use crate::recursive::{Functor, Recursive, RecursiveStruct};

struct Metadata{
    xattrs: Vec<(String,String)>,
}

// structure of the file tree with metadata, no file contents
pub enum FileTree<A> {
    File(Metadata),
    Dir(HashMap<String, A>),
}

impl<A, B> Functor<B> for FileTree<A> {
    type To = FileTree<B>;
    type Unwrapped = A;

    fn fmap_into<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            FileTree::File(x) => FileTree::File(x),
            FileTree::Dir(xs) => {
                let xs = xs.into_iter().map(|(k, v)| (k, f(v))).collect();
                FileTree::Dir(xs)
            }
        }
    }
}

pub type RecursiveFileTree = RecursiveStruct<FileTree<usize>>;

pub struct GrepResult {
    pub path: Vec<String>, // path components
    pub contents: String,
}

impl RecursiveFileTree {
    pub fn depth(self) -> usize {
        self.cata(|node| match node {
            FileTree::Dir(depths) => depths.into_iter().map(|(_k, v)| v).max().unwrap_or(0) + 1,
            _ => 1,
        })
    }

    // return vec of grep results
    pub fn grep(self, substring: &str, ) -> Vec<GrepResult> {
        self.cata(|node: FileTree<Vec<GrepResult>>| match node {
            FileTree::File(contents) => {
                if contents.contains(substring) {
                    vec![GrepResult {
                        path: Vec::new(),
                        contents: contents.to_string(),
                    }]
                } else {
                    Vec::new()
                }
            }
            FileTree::Dir(xs) => xs
                .into_iter()
                .flat_map(|(path_segment, search_results)| {
                    search_results.into_iter().map(move |grep_result| {
                        let mut path = vec![path_segment.clone()];
                        path.extend(grep_result.path);
                        GrepResult {
                            path,
                            contents: grep_result.contents,
                        }
                    })
                })
                .collect(),
        })
    }
}

// todo: grep files in a given commit
