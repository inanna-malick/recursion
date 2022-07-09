mod filetree;

use clap::Parser;
use colored::*;
use filetree::{build::build_file_tree, search::search};
use regex::Regex;
use std::ffi::OsString;

use crate::filetree::depth;

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

    let fs_tree = build_file_tree(".".to_string(), &|path_component| {
        !args.paths_to_ignore.contains(path_component)
    })
    .await?;

    println!("{} {:?}", "sparse filetree depth:".cyan(), depth(&fs_tree));

    // TODO: remove paths to ignore from here entirely and move it to build phase - cleaner that way, runs all the futures in the map, etc
    let grep_res = search(fs_tree, current_dir, &regex).await?;
    for elem in grep_res.into_iter() {
        println!("{} {:?}", "file:".cyan(), elem.path);
        println!("{} {:?}", "permissions".cyan(), elem.metadata.permissions());
        println!("{} {:?}", "modified".cyan(), elem.metadata.modified());
        for (line_num, matching_line) in elem.matching_lines.into_iter() {
            println!(
                "{}:\t{}",
                format!("{:?}:", line_num).magenta(),
                matching_line
            );
        }
        println!("\n");
    }

    Ok(())
}
