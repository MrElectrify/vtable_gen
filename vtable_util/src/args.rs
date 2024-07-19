use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    pub root: PathBuf,
    pub path: String,
}
