use std::{
    collections::HashMap,
    fs::Metadata,
    path::{Path, PathBuf},
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

pub enum FileTreeRef<'a, A> {
    File(&'a std::fs::Metadata),
    Dir(HashMap<&'a str, A>),
}

impl<A, B> Functor<B> for FileTree<A> {
    type To = FileTree<B>;
    type Unwrapped = A;

    fn fmap<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            FileTree::File(x) => FileTree::File(x),
            FileTree::Dir(xs) => {
                let xs = xs.into_iter().map(|(k, v)| (k, f(v))).collect();
                FileTree::Dir(xs)
            }
        }
    }
}

impl<'a, A: Copy + 'a, B: 'a> Functor<B> for &'a FileTree<A> {
    type To = FileTreeRef<'a, B>;
    type Unwrapped = A;

    fn fmap<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            FileTree::File(x) => FileTreeRef::File(x),
            FileTree::Dir(xs) => {
                let xs = xs.iter().map(|(k, v)| (&k[..], f(*v))).collect();
                FileTreeRef::Dir(xs)
            }
        }
    }
}

pub type RecursiveFileTree = RecursiveStruct<FileTree<usize>>;

impl RecursiveFileTree {
    pub fn depth(&self) -> usize {
        self.as_ref().cata(|node: FileTreeRef<usize>| match node {
            FileTreeRef::Dir(depths) => depths.into_iter().map(|(_k, v)| v).max().unwrap_or(0) + 1,
            _ => 1,
        })
    }

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

    pub async fn build(root_path: String) -> std::io::Result<Self> {
        Self::ana_result_async(None, |dir_entry: Option<DirEntry>| {
            async { coalg(&root_path, dir_entry).await }.boxed()
        })
        .await
    }
}

// TODO BETTER NAME LOL
async fn coalg(
    root_path: &str,
    maybe_dir_entry: Option<DirEntry>,
) -> std::io::Result<FileTree<Option<DirEntry>>> {
    println!("visit: {:?}", maybe_dir_entry);
    match maybe_dir_entry {
        None => {
            let entries = process_dir(root_path).await?;
            Ok(FileTree::Dir(entries))
        }
        Some(dir_entry) => {
            let file_type = dir_entry.file_type().await?;
            if file_type.is_dir() {
                let entries = process_dir(dir_entry.path()).await?;
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

async fn process_dir(path: impl AsRef<Path>) -> std::io::Result<HashMap<String, Option<DirEntry>>> {
    let mut entries = HashMap::new();
    // root dir special case
    let mut dirs = tokio::fs::read_dir(path).await?;
    while let Some(next) = dirs.next_entry().await? {
        entries.insert(
            next.file_name().into_string().expect("bad os string"),
            Some(next),
        );
    }

    Ok(entries)
}

// lazy traversal of filetree with path component
type LazilyTraversableFileTree<'a, Res, Err> =
    FileTree<Box<dyn FnOnce(PathBuf) -> BoxFuture<'a, Result<Res, Err>> + Send + Sync + 'a>>;

#[derive(Debug, Clone)]
pub struct GrepResult {
    pub path: PathBuf,
    pub metadata: Metadata,
    pub contents: String,
}

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
