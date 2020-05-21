mod opt;

use hashbrown::{HashMap, HashSet};
use imprint::Imprint;
use opt::Opt;
use std::path::{Path, PathBuf};
use std::{fs, io};
use walkdir::DirEntry;

fn main() -> io::Result<()> {
    let opt = Opt::from_args();
    if opt.compare.is_empty() {
        single_tree(opt.path(), opt.force)
    } else {
        multi_tree(opt.path(), &opt.compare, opt.force)
    }
}

fn single_tree(path: &str, force: bool) -> io::Result<()> {
    let mut files_by_len = HashMap::new();
    for file in list_files(path) {
        let m = file.path().metadata()?;
        files_by_len
            .entry(m.len())
            .or_insert_with(Vec::new)
            .push(file);
    }

    let file_groups = files_by_len
        .into_iter()
        .map(|(_, group)| group)
        .filter(|group| group.len() > 1);

    let mut files_by_imprint = HashMap::new();
    for mut group in file_groups {
        group.sort_by(|a, b| a.path().cmp(b.path()));
        for file in group {
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

    if force {
        remove_duplicates(duplicate_paths)?;
    } else {
        show_duplicates(duplicate_paths);
    }

    Ok(())
}

fn multi_tree(path: &str, compare: &[impl AsRef<Path>], force: bool) -> io::Result<()> {
    let (path_filter, length_filter, imprint_filter) = build_filters(path);

    // Identify external duplicates
    let external_files = compare
        .into_iter()
        .map(list_files)
        .flatten()
        .map(|file| file.path().to_owned())
        .filter(|path| !path_filter.contains(path))
        .filter_map(|file| file.metadata().ok().map(|meta| (file, meta.len())));
    let external_files = external_files
        .filter(|(_, len)| length_filter.contains(len))
        .filter_map(|(path, _)| Imprint::new(&path).ok().map(|imprint| (path, imprint)))
        .filter(|(_, imprint)| imprint_filter.contains(imprint))
        .map(|(path, _)| path);

    if force {
        let result = external_files.into_iter().try_fold(0, |count, path| {
            fs::remove_file(path).map_err(|e| (count, e))?;
            Ok(count + 1)
        });

        match result {
            Ok(count) => println!("Removed {} files", count),
            Err((count, e)) => eprintln!("Removed {} files but failed on {}", count, e),
        }
    } else {
        for path in external_files {
            println!("{}", path.display());
        }
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

fn list_files(root: impl AsRef<Path>) -> impl Iterator<Item = DirEntry> {
    use walkdir::WalkDir;
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
}

fn build_filters(path: &str) -> (HashSet<PathBuf>, HashSet<u64>, HashSet<Imprint>) {
    let files: Vec<_> = list_files(path)
        .map(|file| file.path().to_owned())
        .collect();
    let length_filter: HashSet<_> = files
        .iter()
        .filter_map(|file| file.metadata().ok().map(|meta| meta.len()))
        .collect();
    let imprint_filter: HashSet<_> = files
        .iter()
        .filter_map(|file| Imprint::new(file).ok())
        .collect();

    (files.into_iter().collect(), length_filter, imprint_filter)
}
