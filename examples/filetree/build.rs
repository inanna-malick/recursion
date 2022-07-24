use crate::filetree::{FileTree, RecursiveFileTree};
use futures::FutureExt;
use rust_schemes::recursive::ExpandAsync;
use std::ffi::OsString;
use std::{collections::HashMap, path::Path};
use tokio::fs::DirEntry;

pub async fn build_file_tree<F: for<'x> Fn(&'x OsString) -> bool + Send + Sync>(
    root_path: String,
    filter: &F,
) -> std::io::Result<RecursiveFileTree> {
    RecursiveFileTree::expand_layers_async(None, |dir_entry: Option<DirEntry>| {
        async { build_layer(&root_path, dir_entry, filter).await }.boxed()
    })
    .await
}

async fn build_layer<F: for<'x> Fn(&'x OsString) -> bool + Send + Sync>(
    root_path: &str,
    maybe_dir_entry: Option<DirEntry>,
    filter: &F,
) -> std::io::Result<FileTree<Option<DirEntry>>> {
    match maybe_dir_entry {
        None => {
            let entries = process_dir(root_path, filter).await?;
            Ok(FileTree::Dir(entries))
        }
        Some(dir_entry) => {
            let file_type = dir_entry.file_type().await?;
            if file_type.is_dir() {
                let entries = process_dir(dir_entry.path(), filter).await?;
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

async fn process_dir<F: for<'x> Fn(&'x OsString) -> bool + Send + Sync>(
    path: impl AsRef<Path>,
    filter: &F,
) -> std::io::Result<HashMap<OsString, Option<DirEntry>>> {
    let mut entries = HashMap::new();
    // root dir special case
    // TODO: leaves file handles open and is fucky
    let mut dirs = tokio::fs::read_dir(path).await?;
    while let Some(next) = dirs.next_entry().await? {
        if filter(&next.file_name()) {
            entries.insert(next.file_name(), Some(next));
        }
    }

    Ok(entries)
}
