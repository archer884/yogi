mod opt;

use hashbrown::HashMap;
use imprint::Imprint;
use std::{fs, io, path::PathBuf};
use walkdir::DirEntry;
use opt::Opt;

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    let mut files_by_len = HashMap::new();
    for file in list_files(opt.path()) {
        let metadata = file.path().metadata()?;
        files_by_len
            .entry(metadata.len())
            .or_insert_with(Vec::new)
            .push(file);
    }

    let mut files_by_imprint = HashMap::new();
    for (_, mut files) in files_by_len {
        if files.len() < 2 {
            continue;
        }

        files.sort_by(|a, b| a.path().cmp(b.path()));
        for file in files {
            let imprint = Imprint::new(file.path())?;
            files_by_imprint
                .entry(imprint)
                .or_insert_with(Vec::new)
                .push(file.path().to_owned());
        }
    }

    let mut duplicate_paths: Vec<_> = files_by_imprint
        .into_iter()
        .map(|(_, paths)| paths)
        .filter(|x| x.len() > 1)
        .collect();

    duplicate_paths.sort_by(|left, right| left.cmp(&right));

    if opt.force() {
        remove_duplicates(duplicate_paths)?;
    } else {
        show_duplicates(duplicate_paths);
    }

    Ok(())
}

fn remove_duplicates(grouped_duplicates: impl IntoIterator<Item = Vec<PathBuf>>) -> io::Result<()> {
    let mut count = 0;
    for group in grouped_duplicates {
        count += group
            .into_iter()
            .skip(1)
            .try_fold(0, |count, path| fs::remove_file(path).map(|_| count + 1))?;
    }
    println!("Removed {} files", count);
    Ok(())
}

fn show_duplicates(grouped_duplicates: impl IntoIterator<Item = Vec<PathBuf>>) {
    for group in grouped_duplicates {
        let mut paths = group.into_iter();
        if let Some(path) = paths.next() {
            println!("Path: {}", path.display());
        }
        paths.for_each(|path| println!("    {}", path.display()));
    }
}

fn list_files(root: &str) -> impl Iterator<Item = DirEntry> {
    use walkdir::WalkDir;
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
}
