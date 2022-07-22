pub mod build;
pub mod search;

use schemes::{functor::Functor, recursive_tree::block_allocation::ArenaIndex};
use schemes::recursive::Foldable;
use schemes::recursive_tree::RecursiveTree;
use std::{collections::HashMap, ffi::OsString};

// structure of the file tree with metadata, no file contents, files do not each own their full path b/c that's too much overhead
pub enum FileTree<A> {
    File(std::fs::Metadata),
    Dir(HashMap<OsString, A>),
}

pub enum FileTreeRef<'a, A> {
    File(&'a std::fs::Metadata),
    Dir(HashMap<&'a OsString, A>),
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
                let xs = xs.iter().map(|(k, v)| (k, f(*v))).collect();
                FileTreeRef::Dir(xs)
            }
        }
    }
}

pub type RecursiveFileTree = RecursiveTree<FileTree<ArenaIndex>, ArenaIndex>;

// some utility functions over FileTreeRef, to show how using borrowed data works

/// calculate the depth of a file
pub fn depth(tree: &RecursiveFileTree) -> usize {
    tree.as_ref().fold_layers(|node: FileTreeRef<usize>| match node {
        FileTreeRef::Dir(depths) => depths.into_iter().map(|(_k, v)| v).max().unwrap_or(0) + 1,
        _ => 1,
    })
}
