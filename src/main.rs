use imprint::{Imprint, Metadata};
use std::collections::HashMap;
use std::convert::TryInto;
use std::{fs, io};
use structopt::StructOpt;
use walkdir::DirEntry;

/// Examine a directory for duplicated files and remove them.
#[derive(Clone, Debug, StructOpt)]
struct Opt {
    /// The root path to be examined
    path: String,

    /// Remove duplicate files
    #[structopt(short = "f", long = "force")]
    force: bool,
}

fn main() -> io::Result<()> {
    let Opt { path, force } = Opt::from_args();

    let mut files_by_len = HashMap::new();
    for file in list_files(&path) {
        let metadata = Metadata::from_path(file.path())?;
        files_by_len
            .entry(metadata.len())
            .or_insert_with(Vec::new)
            .push(metadata);
    }

    let mut files_by_imprint = HashMap::new();
    for (_, mut files) in files_by_len {
        if files.len() < 2 {
            continue;
        }

        files.sort_by(|a, b| a.path().cmp(b.path()));
        for file in files {
            let imprint: Imprint = file.try_into()?;
            let path = imprint.path().to_owned();
            files_by_imprint
                .entry(imprint)
                .or_insert_with(Vec::new)
                .push(path);
        }
    }

    let mut duplicate_paths: Vec<_> = files_by_imprint
        .into_iter()
        .map(|(_, paths)| paths)
        .filter(|x| x.len() > 1)
        .collect();

    duplicate_paths.sort_by(|left, right| left.cmp(&right));
    for path_set in duplicate_paths {
        let mut paths = path_set.into_iter().map(|path| {
            let display = path.display().to_string();
            (path, display)
        });

        if force {
            for (path, display) in paths.skip(1) {
                fs::remove_file(path)?;
                println!("Removed: {}", display);
            }
        } else if let Some((_, display)) = paths.next() {
            println!("Path: {}", display);
            for (_, display) in paths {
                println!("    {}", display);
            }
        }
    }

    Ok(())
}

fn list_files(root: &str) -> impl Iterator<Item = DirEntry> {
    use walkdir::WalkDir;
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
}
