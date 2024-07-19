use std::collections::{BTreeSet, HashMap, VecDeque};
use std::fs::{OpenOptions, read_to_string};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use darling::{FromMeta, ToTokens};
use itertools::Itertools;
use regex::Regex;

use crate::util::walk_dir;

/// The regex for a class definition and its dependents.
const CLASS_DEFINITION_REGEX: &str = "struct ([\\w]*): ([\\w,<>\n ]*)\\{";
/// The regex for the `vtable_gen` use statement.
const VTABLE_GEN_REGEX: &str = "use vtable_gen::[\\{]?[\\w,\n ]*[\\{]?;\n";

#[derive(Debug)]
pub struct DependencyTree {
    dependencies: HashMap<String, Vec<String>>,
    root: PathBuf,
    class_regex: Regex,
    base_path: syn::Path,
}

impl DependencyTree {
    /// Builds a dependency tree from a root.
    pub fn from_path(root: &Path, base_path: &str) -> Self {
        let mut dependencies = HashMap::default();
        let class_regex = Regex::new(CLASS_DEFINITION_REGEX).expect("failed to parse regex");

        // open each file and parse out the class tree
        for entry in walk_dir(root) {
            // read the file
            let entry = entry.expect("failed to inspect entry");
            let path = entry.path();
            let contents = read_to_string(path).expect("failed to read file");

            // parse out the struct definitions
            for (name, bases) in Self::capture_classes(&class_regex, &contents) {
                if dependencies.insert(name.to_owned(), bases).is_some() {
                    panic!("Duplicate definition for type {name} in file {path:?}")
                }
            }
        }

        Self {
            dependencies,
            root: root.to_path_buf(),
            class_regex,
            base_path: syn::Path::from_string(base_path).expect("failed to parse base path"),
        }
    }

    /// Adds necessary uses for each type.
    pub fn add_uses(&self) {
        let vtable_regex = Regex::new(VTABLE_GEN_REGEX).expect("failed to parse vtable regex");
        for entry in walk_dir(&self.root) {
            // read the file
            let entry = entry.expect("failed to inspect entry");
            let path = entry.path();

            // open the file in R/W mode
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .truncate(false)
                .open(path)
                .expect("failed to open file");

            // parse the classes
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("failed to read file");

            // find the vtable gen use statement
            let Some(vtable_gen_stmt) = vtable_regex.find(&contents) else {
                continue;
            };

            // capture all uses
            let uses: BTreeSet<String> = Self::capture_classes(&self.class_regex, &contents)
                .flat_map(|(name, _)| self.dependencies(&name))
                .collect();
            if uses.is_empty() {
                continue;
            }

            let uses = format!(
                "use {}::{{{}}};\n",
                self.base_path.to_token_stream(),
                uses.iter().map(|u| format!("{u}VTable")).join(", ")
            );

            // seek to the beginning of the statement and inject our statement
            println!(
                "Adding to file {path:?} as pos {}: {uses:?}",
                vtable_gen_stmt.start()
            );

            contents.insert_str(vtable_gen_stmt.start(), &uses);

            // seek to the start
            file.seek(SeekFrom::Start(0)).expect("failed to seek file");

            // output the statement
            file.write_all(contents.as_bytes())
                .expect("failed to inject the use statement");
        }
    }

    /// Produces a vector containing all of a class's bases.
    pub fn dependencies(&self, ty: &str) -> Vec<String> {
        let mut dependencies = Vec::new();
        let mut types_to_check = VecDeque::new();
        types_to_check.push_back(ty.to_owned());

        while let Some(type_to_check) = types_to_check.pop_front() {
            let Some(ty_dependencies) = self.dependencies.get(&type_to_check) else {
                continue;
            };

            dependencies.extend(ty_dependencies.iter().cloned());
            types_to_check.extend(ty_dependencies.iter().cloned());
        }

        dependencies
    }

    fn capture_classes<'a, 'b: 'a>(
        class_regex: &'a Regex,
        contents: &'b str,
    ) -> impl Iterator<Item = (String, Vec<String>)> + 'a {
        class_regex.captures_iter(contents).map(move |capture| {
            let name = capture[1].to_owned();
            let bases: String = capture[2]
                .chars()
                .filter(|c| {
                    c.is_ascii_alphanumeric() || c == &'<' || c == &'>' || c == &'_' || c == &','
                })
                .collect();
            let bases = bases.split_terminator(",").map(str::to_owned).collect();

            (name, bases)
        })
    }
}
