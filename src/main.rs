mod fingerprint;

use std::fs;
use fingerprint::*;
use std::collections::HashMap;
use walkdir::DirEntry;
use structopt::StructOpt;

/// Examine a directory for duplicated files and remove them.
#[derive(Clone, Debug, StructOpt)]
struct Opt {
    /// The root path to be examined
    path: String,

    /// Remove duplicate files
    #[structopt(short = "f", long = "force")]
    force: bool,    
}


fn main() {
    let Opt{ path, force } = Opt::from_args();
    let mut files_by_partial_fingerprint = HashMap::new();
    for file in list_files(&path) {
        if let Ok(partial) = PartialFingerprint::from_path(file.path()) {
            files_by_partial_fingerprint
                .entry(partial)
                .or_insert_with(Vec::new)
                .push(file.path().to_owned());
        }
    }

    let mut files_by_fingerprint = HashMap::new();
    for (_, mut paths) in files_by_partial_fingerprint {
        if paths.len() < 2 {
            continue;
        }

        paths.sort();
        for path in paths {
            if let Ok(fingerprint) = Fingerprint::from_path(&path) {
                files_by_fingerprint
                    .entry(fingerprint)
                    .or_insert_with(Vec::new)
                    .push(path);
            }
        }
    }

    let mut duplicate_paths: Vec<_> = files_by_fingerprint
        .into_iter()
        .map(|(_, paths)| paths)
        .filter(|x| x.len() > 1)
        .collect();

    duplicate_paths.sort_by(|left, right| left.cmp(&right));
    for path_set in duplicate_paths {
        let paths: Vec<_> = path_set
            .into_iter()
            .map(|path| {
                let display = path.display().to_string();
                (path, display)
            })
            .collect();

        for (path, display) in paths.into_iter().skip(1) {
            if force {
                let _ = fs::remove_file(path);
                println!("Removed: {}", display);
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
