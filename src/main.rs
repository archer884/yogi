#![feature(box_syntax)]

extern crate digest;
extern crate picnic;
extern crate sha2;
extern crate walkdir;

mod fingerprint;

use fingerprint::*;
use picnic::{Dictionary, Ranker};
use std::cmp::Reverse;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::process;
use walkdir::DirEntry;

fn main() {
    let root = match env::args().nth(1) {
        Some(root) => root,
        None => {
            eprintln!("Please provide a root directory");
            process::exit(1);
        }
    };

    let mut files_by_partial_fingerprint = HashMap::new();
    for file in list_files(&root) {
        if let Ok(partial) = PartialFingerprint::from_path(file.path()) {
            files_by_partial_fingerprint
                .entry(partial)
                .or_insert_with(Vec::new)
                .push(file.path().to_owned());
        }
    }

    let mut files_by_fingerprint = HashMap::new();
    for (_, paths) in files_by_partial_fingerprint {
        if paths.len() < 2 {
            continue;
        }

        for path in paths {
            if let Ok(fingerprint) = Fingerprint::from_path(&path) {
                files_by_fingerprint
                    .entry(fingerprint)
                    .or_insert_with(Vec::new)
                    .push(path);
            }
        }
    }

    let dictionary = include_str!("../resource/enable1.txt");
    let dictionary = SetDictionary(dictionary.split_whitespace().collect());
    let ranker = Ranker::new(dictionary);

    for (_, mut paths) in files_by_fingerprint {
        if paths.len() > 1 {
            let mut paths: Vec<_> = paths
                .into_iter()
                .map(|path| path.display().to_string())
                .collect();

            // We reverse this comparison because it is desirable to retain files with
            // more descriptive names rather than files with less descriptive names.
            // Descriptive names are usually longer.
            paths.sort_by_key(|path| Reverse(ranker.rank(path)));

            for path in paths.into_iter().skip(1) {
                println!("{}", path);
            }
        }
    }
}

fn list_files(root: &str) -> impl Iterator<Item = DirEntry> {
    use walkdir::WalkDir;
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
}

struct SetDictionary<'a>(HashSet<&'a str>);

impl<'a> Dictionary for SetDictionary<'a> {
    fn contains(&self, s: &str) -> bool {
        self.0.contains(s)
    }
}
