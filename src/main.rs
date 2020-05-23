mod opt;

use hashbrown::{HashMap, HashSet};
use imprint::Imprint;
use opt::Opt;
use std::path::{Path, PathBuf};
use std::{fs, io};
use walkdir::{DirEntry, WalkDir};

struct ExclusionFilter {
    exclude: PathBuf,
}

impl ExclusionFilter {
    fn from_path(path: impl AsRef<Path>) -> Self {
        let exclude = fs::canonicalize(path.as_ref()).unwrap_or_else(|_| path.as_ref().into());
        Self { exclude }
    }

    fn is_valid(&self, entry: &DirEntry) -> bool {
        use std::borrow::Cow;
        !entry.file_type().is_dir() || {
            let path = fs::canonicalize(entry.path())
                .map(Cow::from)
                .unwrap_or_else(|_| Cow::from(entry.path()));
            self.exclude != path
        }
    }
}

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

    let mut grouped_duplicates: Vec<_> = files_by_imprint
        .into_iter()
        .map(|(_, paths)| paths)
        .filter(|x| x.len() > 1)
        .collect();

    grouped_duplicates.sort_by(|left, right| left.cmp(&right));

    if force {
        let result = grouped_duplicates
            .into_iter()
            .map(|group| group.into_iter().skip(1))
            .flatten()
            .try_fold(0, |count, path| {
                fs::remove_file(path).map_err(|e| (count, e))?;
                Ok(count + 1)
            });

        match result {
            Ok(count) => println!("Removed {} files", count),
            Err((count, e)) => eprintln!("Removed {} files but failed on {}", count, e),
        }
    } else {
        for group in grouped_duplicates {
            let (primary, duplicates) = group
                .split_first()
                .expect("We should already have filtered out small groups");
            println!("Path: {}", primary.display());
            duplicates
                .into_iter()
                .for_each(|path| println!("   {}", path.display()));
        }
    }

    Ok(())
}

fn multi_tree(path: &str, compare: &[impl AsRef<Path>], force: bool) -> io::Result<()> {
    let (length_filter, imprint_filter) = build_filters(path);

    // Identify external duplicates
    let external_files = compare
        .into_iter()
        .map(|root| list_files_with_exclusion(root, path))
        .flatten()
        .map(|file| file.path().to_owned())
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

fn list_files(root: impl AsRef<Path>) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
}

fn list_files_with_exclusion<'a>(
    root: impl AsRef<Path>,
    exclude: impl AsRef<Path> + 'a,
) -> impl Iterator<Item = DirEntry> + 'a {
    let filter = ExclusionFilter::from_path(exclude.as_ref());
    WalkDir::new(root)
        .into_iter()
        .filter_entry(move |entry| filter.is_valid(entry))
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
}

fn build_filters(path: &str) -> (HashSet<u64>, HashSet<Imprint>) {
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

    (length_filter, imprint_filter)
}
