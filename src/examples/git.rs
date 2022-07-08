use std::{
    collections::HashMap,
    fs::{self, Metadata},
    path::Path,
};

use tokio::fs::DirEntry;

use futures::{future::BoxFuture, FutureExt};

use crate::recursive::{CoRecursiveAsync, Functor, Recursive, RecursiveStruct};

type PathComponent = String;

// structure of the file tree with metadata, no file contents, files do not each own their full path b/c that's too much overhead
pub enum FileTree<A> {
    File(std::fs::Metadata),
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

#[derive(Debug, Clone)]
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
        root_dir: String,
        substring: &'a str,
        filter: &'a F,
    ) -> BoxFuture<'a, Vec<GrepResult>> {
        println!("grep");
        let f = self
            .cata(move |node| Box::new(move |path| alg::<'a, _>(node, path, substring, filter)));

        f(vec![root_dir])
    }



    pub async fn build(root_path: String) -> std::io::Result<Self> {
        println!("build {:?}", root_path);
        Self::ana_result_async(None, |dir_entry: Option<DirEntry>| {
            async { coalg(&root_path, dir_entry).await }.boxed()
        })
        .await
    }
}

// TODO BETTER NAME LOL - or two functions?
async fn coalg(
    root_path: &str,
    maybe_dir_entry: Option<DirEntry>,
) -> std::io::Result<FileTree<Option<DirEntry>>> {
    println!("visit: {:?}", maybe_dir_entry);
    match maybe_dir_entry {
        None => {
            println!("root case");
            let mut entries = HashMap::new();
            // root dir special case
            let mut dirs = tokio::fs::read_dir(root_path).await?;
            while let Some(next) = dirs.next_entry().await? {
                entries.insert(
                    next.file_name().into_string().expect("bad os string"),
                    Some(next),
                );
            }

            Ok(FileTree::Dir(entries))
        }
        Some(dir_entry) => {
            let file_type = dir_entry.file_type().await?;
            if file_type.is_dir() {
                // note: duplicated code :(
                let mut entries = HashMap::new();
                // root dir special case
                let mut dirs = tokio::fs::read_dir(dir_entry.path()).await?;
                while let Some(next) = dirs.next_entry().await? {
                    entries.insert(
                        next.file_name().into_string().expect("bad os string"),
                        Some(next),
                    );
                }

                Ok(FileTree::Dir(entries))
            } else if file_type.is_file() {
                let metadata = dir_entry.metadata().await?;
                Ok(FileTree::File(metadata))
            } else {
                panic!("only dirs and files currently supported")
            }
        }
    }
}

// TODO BETTER NAME LOL
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
                let path_joined = path.join("/");
                println!("visit file with path {:?} for grep", path_joined);
                let contents = tokio::fs::read_to_string(path_joined)
                    .await
                    .expect("todo short circuit here");
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
