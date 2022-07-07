use std::{collections::HashMap, fs};

use futures::{future::BoxFuture, FutureExt};

use crate::recursive::{Functor, Recursive, RecursiveStruct};

pub struct Metadata {
    pub xattrs: Vec<(String, String)>,
}

type PathComponent = String;

// structure of the file tree with metadata, no file contents
pub enum FileTree<A> {
    File(Metadata),
    Dir(HashMap<PathComponent, A>),
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
    pub path: Vec<PathComponent>, // path components
    pub metadata: Metadata,
    pub contents: String,
}

impl RecursiveFileTree {
    pub fn depth(self) -> usize {
        self.cata(|node| match node {
            FileTree::Dir(depths) => depths.into_iter().map(|(_k, v)| v).max().unwrap_or(0) + 1,
            _ => 1,
        })
    }

    // return vec of grep results, with short circuits, via magic
    // note: dear lord, this is absolutely atrocious, but it works. hahahahahahahahhahafewhafe
    pub fn grep<'a, F: Fn(&[PathComponent]) -> bool + Send + Sync + 'a>(
        self,
        substring: &'a str,
        filter: &'a F,
    ) -> BoxFuture<'a, Vec<GrepResult>> {
        let f = self
            .cata(move |node| Box::new(move |path| alg::<'a, _>(node, path, substring, filter)));

        f(Vec::new())
    }
}

fn alg<'a, F: Fn(&[PathComponent]) -> bool + Send + Sync + 'a>(
    node: FileTree<
        Box<dyn FnOnce(Vec<PathComponent>) -> BoxFuture<'a, Vec<GrepResult>> + Send + Sync + 'a>,
    >,
    path: Vec<PathComponent>,
    substring: &'a str,
    filter: &'a F,
) -> BoxFuture<'a, Vec<GrepResult>> {
    async move {
        match node {
            FileTree::File(metadata) => {
                let contents = fs::read_to_string(path.join("/")).expect("todo short circuit here");
                if contents.contains(substring) {
                    vec![GrepResult {
                        path,
                        metadata,
                        contents: contents,
                    }]
                } else {
                    Vec::new()
                }
            }
            FileTree::Dir(search_results_futs) => {
                let mut all_results = Vec::new();
                for (path_component, search_result_fut) in search_results_futs.into_iter() {
                    let mut child_path = path.clone();
                    child_path.push(path_component);
                    if filter(&child_path[..]) {
                        // only expand child search branch if filter fn returns true
                        let search_result = search_result_fut(child_path).await;
                        all_results.extend(search_result.into_iter());
                    }
                }
                all_results
            }
        }
    }
    .boxed()
}

// todo: grep files in a given commit
