use recursive::examples::filetree::RecursiveFileTree;

// build a recursive tree of filesystem state (dirs and files with metadata only) then
// lazily traverse it grep-style to return
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let fs_tree = RecursiveFileTree::build(".".to_string()).await?;
    let grep_res = fs_tree
        .grep(".".to_string(), "Expr", &|path| {
            !(path.contains(&"target".to_string()) || path.contains(&".git".to_string()))
        })
        .await?;
    for elem in grep_res.into_iter() {
        println!("grep res: {:?}", elem);
    }

    Ok(())
}
