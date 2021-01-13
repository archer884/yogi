use std::{
    fs, io,
    path::{Path, PathBuf},
};

use bumpalo::Bump;
use fmtsize::{Conventional, FmtSize};
use hashbrown::{HashMap, HashSet};
use imprint::Imprint;
use walkdir::{DirEntry, WalkDir};

use crate::{Meta, Metacache};

type ImprintMap<'a> = HashMap<Imprint, Vec<&'a Path>>;

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

fn external_paths<'a>(
    exclude: &'a str,
    include: &'a [impl AsRef<Path>],
    path_src: &'a Bump,
) -> impl Iterator<Item = &'a Path> + 'a {
    include
        .iter()
        .flat_map(move |path| list_files_with_exclusion(path, exclude))
        .map(move |entry| &**path_src.alloc(entry.path().to_owned()))
}

pub fn process(
    path: &str,
    compare: &[impl AsRef<Path>],
    force: bool,
    recurse: bool,
) -> io::Result<()> {
    use hashbrown::hash_map::Entry;

    let paths = Bump::new();
    let mut cache = Metacache::new();
    let (length_filter, mut conflicts) = initialize_maps(path, &paths, &mut cache, recurse)?;

    for path in external_paths(path, compare, &paths) {
        let meta: Meta = path.metadata()?.into();
        if length_filter.contains(&meta.len) {
            let imprint = Imprint::new(path)?;
            if let Entry::Occupied(mut e) = conflicts.entry(imprint) {
                e.get_mut().push(path);
                cache.insert(path, meta);
            }
        }
    }

    let conflicts = conflicts.into_iter().filter(|x| x.1.len() > 1);

    if force {
        let (count, size) = super::deconflict(conflicts, &cache)?;
        println!("Removed {} files ({})", count, size.fmt_size(Conventional));
    } else {
        super::pretty_print_conflicts(conflicts, &cache)?;
    }

    Ok(())
}

fn initialize_maps<'a>(
    path: &str,
    path_src: &'a Bump,
    metacache: &mut Metacache<'a>,
    recurse: bool,
) -> io::Result<(HashSet<u64>, ImprintMap<'a>)> {
    let mut lengths = HashSet::new();
    let mut conflicts = HashMap::new();
    for entry in super::list_entries(path, recurse) {
        let path = &**path_src.alloc(entry.path().to_owned());
        let meta: Meta = path.metadata()?.into();
        lengths.insert(meta.len);
        metacache.insert(path, meta);
        conflicts.insert(Imprint::new(path)?, vec![path]);
    }
    Ok((lengths, conflicts))
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
