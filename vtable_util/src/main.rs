use clap::Parser;

use crate::args::Args;
use crate::dependency_tree::DependencyTree;

mod args;
mod dependency_tree;
mod util;

fn main() {
    let args = Args::parse();
    let tree = DependencyTree::from_path(&args.root, &args.path);
    tree.add_uses();
}
