mod fingerprint;

use std::fs;
use fingerprint::*;
use std::collections::HashMap;
use std::env;
use std::process;
use walkdir::DirEntry;
use std::path::PathBuf;

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

    let duplicate_paths = files_by_fingerprint
        .into_iter()
        .map(|(_, paths)| paths)
        .filter(|x| x.len() > 1);

    for path_set in duplicate_paths {
        let paths: Vec<_> = path_set
            .into_iter()
            .map(|path| {
                let display = path.display().to_string();
                (path, display)
            })
            .collect();

        if let Some(retain) = get_selection(&paths) {
            let paths_to_remove = paths
                .into_iter()
                .enumerate()
                .filter(|&(idx, _)| retain != idx)
                .map(|(_, x)| x);

            for (path, display) in paths_to_remove {
                let _ = fs::remove_file(path);
                println!("Removed: {}", display);
            }
        }
    }
}

fn get_selection(paths: &[(PathBuf, String)]) -> Option<usize> {
    println!("Select a file to keep:");
    for (idx, path) in paths.into_iter().map(|(_, x)| x).enumerate() {
        println!("{}: {}", idx, path);
    }
    read_number()
}

fn read_number() -> Option<usize> {
    use std::io;
    let mut buf = String::new();
    let handle = io::stdin();
    handle.read_line(&mut buf).ok()?;
    buf.parse().ok()
}

fn list_files(root: &str) -> impl Iterator<Item = DirEntry> {
    use walkdir::WalkDir;
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
}
