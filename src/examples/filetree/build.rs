use crate::examples::filetree::{FileTree, RecursiveFileTree};
use crate::recursive::CoRecursiveAsync;
use futures::FutureExt;
use std::ffi::OsString;
use std::{collections::HashMap, path::Path};
use tokio::fs::DirEntry;

impl RecursiveFileTree {
    pub async fn build(root_path: String) -> std::io::Result<Self> {
        Self::ana_result_async(None, |dir_entry: Option<DirEntry>| {
            async { build_layer(&root_path, dir_entry).await }.boxed()
        })
        .await
    }
}

async fn build_layer(
    root_path: &str,
    maybe_dir_entry: Option<DirEntry>,
) -> std::io::Result<FileTree<Option<DirEntry>>> {
    // println!("building file tree, visit: {:?}", maybe_dir_entry);
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

async fn process_dir(path: impl AsRef<Path>) -> std::io::Result<HashMap<OsString, Option<DirEntry>>> {
    let mut entries = HashMap::new();
    // root dir special case
    let mut dirs = tokio::fs::read_dir(path).await?;
    while let Some(next) = dirs.next_entry().await? {
        entries.insert(
            next.file_name(),
            Some(next),
        );
    }

    Ok(entries)
}
