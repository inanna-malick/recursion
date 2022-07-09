use std::ffi::{ OsString};

use recursive::examples::filetree::RecursiveFileTree;
use clap::Parser;
use colored::*;
use regex::Regex;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the p&erson to greet
    #[clap(short, long)]
    regex: String,

    /// paths to filter out
    #[clap(short, long)]
    paths_to_ignore: Vec<OsString>,
}


// build a recursive tree of filesystem state (dirs and files with metadata only) then
// lazily traverse it grep-style to return
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let regex = Regex::new(&args.regex).unwrap();

    let current_dir = std::env::current_dir()?;

    let fs_tree = RecursiveFileTree::build(".".to_string()).await?;

    println!("{} {}", "fs tree depth:".cyan(), fs_tree.depth());

    let grep_res = fs_tree
        .grep(current_dir, &regex, &|path| {
            !path.components().any(|component| {
                args.paths_to_ignore.iter().any( |ignored| component.as_os_str() == ignored)
            })
        })
        .await?;
    for elem in grep_res.into_iter() {
        println!("{} {:?}", "file:".cyan(), elem.path);
        println!("{} {:?}", "permissions".cyan(), elem.metadata.permissions());
        println!("{} {:?}", "modified".cyan(), elem.metadata.modified());
        for (line_num, matching_line) in elem.matching_lines.into_iter() {
            println!("{}:\t{}", format!("{:?}:", line_num).magenta(), matching_line);
        }
        println!("\n");
    }

    Ok(())
}
