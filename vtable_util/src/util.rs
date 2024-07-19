use std::path::Path;

use walkdir::{DirEntry, WalkDir};

/// Produces a recursive iterator over all files within a directory.
pub fn walk_dir<P: AsRef<Path>>(root: P) -> impl Iterator<Item = walkdir::Result<DirEntry>> {
    // filter out directories and failures
    WalkDir::new(root).into_iter().filter(|entry| match entry {
        Ok(entry) => !entry.path().is_dir(),
        _ => true,
    })
}
