use std::{collections::HashMap, default};

use crate::recursive::{Functor, Recursive, RecursiveStruct};

// idk hash and etc
pub enum FileTree<A> {
    Blob(String),
    BinaryBlob(Vec<u8>),
    Dir(HashMap<String, A>),
}





impl<A, B> Functor<B> for FileTree<A> {
    type To = FileTree<B>;
    type Unwrapped = A;

    fn fmap_into<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            FileTree::Blob(x) => FileTree::Blob(x),
            FileTree::BinaryBlob(x) => FileTree::BinaryBlob(x),
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
            FileTree::Blob(_) => 1,
            FileTree::BinaryBlob(_) => 1,
            FileTree::Dir(depths) => depths.into_iter().map(|(_k, v)| v).max().unwrap_or(0) + 1,
        })
    }

    // return vec of all file bodies that contain provided substring
    pub fn grep_simple(self, substring: &str) -> Vec<String> {
        let mut res = Vec::new();

        self.cata(|node| match node {
            FileTree::Blob(contents) => {
                if contents.contains(substring) {
                    res.push(contents)
                }
            }
            _ => {}
        });

        res
    }

    // return vec of grep results
    pub fn grep(self, substring: &str) -> Vec<GrepResult> {
        self.cata(|node: FileTree<Vec<GrepResult>>| match node {
            FileTree::Blob(contents) => {
                if contents.contains(substring) {
                    vec![GrepResult {
                        path: Vec::new(),
                        contents: contents.to_string(),
                    }]
                } else {
                    Vec::new()
                }
            }
            FileTree::BinaryBlob(_) => Vec::new(),
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
