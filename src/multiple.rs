use std::{
    borrow::Borrow,
    fs,
    io::{self, Write},
    path::Path,
};

use bumpalo::Bump;
use fmtsize::{Conventional, FmtSize};
use hashbrown::{HashMap, HashSet};
use imprint::Imprint;

use crate::{Meta, Metacache};

#[derive(Clone, Debug, Default)]
struct Conflict<'a> {
    base_files: Vec<&'a Path>,
    compare_files: Vec<&'a Path>,
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

    let base_files: HashSet<_> = super::list_entries(path, recurse)
        .filter_map(|entry| entry.path().canonicalize().ok())
        .map(|entry| &**paths.alloc(entry))
        .collect();
    let compare_files: HashSet<_> = compare
        .iter()
        .flat_map(|path| super::list_entries(path, recurse))
        .filter_map(|entry| entry.path().canonicalize().ok())
        .map(|entry| &**paths.alloc(entry))
        .collect();

    let compare_files: Vec<_> = compare_files.difference(&base_files).copied().collect();
    let base_files_by_length: HashMap<_, _> = by_length(base_files.iter().copied())?;
    let mut files_by_imprint: HashMap<Imprint, Conflict> = HashMap::new();

    // Here be dragons.

    // Basically, the first thing we do is populate files_by_imprint with any *potential* conflicts
    // (as determined by file length) from the base file list. The key for this process is the file
    // imprint, but the value is a struct called Conflict and all base file paths are inserted here
    // onto the conflict.base_files member. This process is performed for each path in the
    // comparison set.

    for path in compare_files {
        let meta: Meta = path.metadata()?.into();
        if let Some(potential_conflicts) = base_files_by_length.get(&meta.len) {
            let imprints = potential_conflicts
                .iter()
                .filter_map(|&path| Imprint::new(path).ok().map(|imprint| (path, imprint)));
            for (base_path, imprint) in imprints {
                files_by_imprint
                    .entry(imprint)
                    .or_default()
                    .base_files
                    .push(base_path);
            }
        }

        // Now we're on to step two, which is to populate only occupied entries with the paths of
        // files with matching imprints from the comparison set. In theory, only the
        // conflict.compare_files member is modified here, and only files from the set of
        // compared paths rather than files from the base path. However, when pretty-printed,
        // these results pretty much ALWAYS look WEIRD. Specifically, they look as though the base
        // file and compare file have the same filename. (As of May 26, 2022.)

        let imprint = Imprint::new(path)?;
        if let Entry::Occupied(mut conflicts) = files_by_imprint.entry(imprint) {
            conflicts.get_mut().compare_files.push(path);
            cache.insert(path, meta);
        }
    }

    let conflicts = files_by_imprint
        .into_iter()
        .filter(|entry| !entry.1.compare_files.is_empty());

    if force {
        let mut count = 0usize;
        let mut size = 0u64;
        for path in conflicts
            .into_iter()
            .flat_map(|entry| entry.1.compare_files)
        {
            fs::remove_file(path)?;
            count += 1;
            size += cache.get(path).map(|meta| meta.len).unwrap_or_default();
        }
        println!("Removed {} files ({})", count, size.fmt_size(Conventional));
    } else {
        pretty_print_conflicts(conflicts, &cache)?;
    }

    Ok(())
}

fn pretty_print_conflicts<'a>(
    groups: impl IntoIterator<Item = (Imprint, Conflict<'a>)>,
    cache: &Metacache,
) -> io::Result<()> {
    let mut handle = io::stdout().lock();
    let mut count = 0;
    let mut size = 0;

    for (imprint, conflict) in groups {
        count += conflict.compare_files.len();
        size += conflict.compare_files.len() as u64
            * conflict
                .compare_files
                .first()
                .and_then(|&path| cache.get(path).map(|cx| cx.len))
                .unwrap_or_default();

        writeln!(
            handle,
            "{}\n----------------------------------------------------------------",
            imprint,
        )?;

        for &path in &conflict.base_files {
            writeln!(handle, "{}", path.file_name().unwrap().to_string_lossy())?;
        }

        writeln!(
            handle,
            "================================================================",
        )?;

        for &path in &conflict.compare_files {
            writeln!(handle, "{}", path.file_name().unwrap().to_string_lossy())?;
        }
        writeln!(handle)?;
    }

    writeln!(
        handle,
        "{} duplicates ({})",
        count,
        size.fmt_size(Conventional)
    )?;

    Ok(())
}

fn by_length<'a, I>(paths: I) -> io::Result<HashMap<u64, Vec<&'a Path>>>
where
    I: IntoIterator<Item = &'a Path> + 'a,
{
    let mut map = HashMap::new();
    for path in paths {
        let meta = fs::metadata(path.borrow())?;
        map.entry(meta.len())
            .or_insert_with(Vec::new)
            .push(path.borrow());
    }
    Ok(map)
}
