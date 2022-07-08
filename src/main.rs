use std::ffi::OsStr;

use recursive::examples::filetree::RecursiveFileTree;

// build a recursive tree of filesystem state (dirs and files with metadata only) then
// lazily traverse it grep-style to return
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let current_dir = std::env::current_dir()?;

    let fs_tree = RecursiveFileTree::build(".".to_string()).await?;
    let grep_res = fs_tree
        .grep(current_dir, "Expr", &|path| {
            let git_dir_component = OsStr::new(".git");
            let target_dir_component = OsStr::new("target");
            !path.components().any(|component| {
                component.as_os_str() == git_dir_component
                    || component.as_os_str() == target_dir_component
            })
        })
        .await?;
    for elem in grep_res.into_iter() {
        println!("grep res: {:?}", elem);
    }

    Ok(())
}
