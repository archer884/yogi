use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use bumpalo::Bump;
use fmtsize::{Conventional, FmtSize};
use hashbrown::{HashMap, HashSet, hash_map::EntryRef};
use imprint::Imprint;

use crate::meta::{Meta, Metacache};

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
    ignore: &[&Path],
) -> io::Result<()> {
    let paths = Bump::new();
    let mut context = Context {
        root: path,
        compare_to: compare,
        paths: &paths,
        cache: Metacache::new(),
        ignore,
    };

    let conflicts = get_conflicts(&mut context, recurse)?;

    if force {
        let mut count = 0usize;
        let mut size = 0u64;
        for path in conflicts
            .into_iter()
            .flat_map(|entry| entry.1.compare_files)
        {
            fs::remove_file(path)?;
            count += 1;
            size += context
                .cache
                .get(path)
                .map(|meta| meta.len)
                .unwrap_or_default();
        }
        println!("Removed {} files ({})", count, size.fmt_size(Conventional));
    } else {
        pretty_print_conflicts(conflicts, &context.cache)?;
    }

    Ok(())
}

#[derive(Debug)]
struct Context<'a, T> {
    root: &'a str,
    compare_to: &'a [T],
    paths: &'a Bump,
    cache: Metacache<'a>,
    ignore: &'a [&'a Path],
}

fn get_conflicts<'a, T: AsRef<Path>>(
    context: &mut Context<'a, T>,
    recurse: bool,
) -> io::Result<impl Iterator<Item = (Imprint, Conflict<'a>)> + use<'a, T>> {
    let base_files: HashSet<&Path> = super::list_entries(context.root, recurse, context.ignore)
        .filter_map(|entry| entry.path().canonicalize().ok())
        .map(|entry| &**context.paths.alloc(entry))
        .collect();

    // We're going to attempt to prevent files in the basic set from appearing in this set.
    let compare_files: HashSet<_> = context
        .compare_to
        .iter()
        .flat_map(|path| super::list_entries(path, recurse, context.ignore))
        .filter_map(|entry| entry.path().canonicalize().ok())
        .map(|entry| &**context.paths.alloc(entry))
        .collect();

    let compare_files: Vec<_> = compare_files.difference(&base_files).copied().collect();
    let base_files_by_length: HashMap<_, _> = by_length(base_files.into_iter())?;
    let mut files_by_imprint: HashMap<Imprint, Conflict> = HashMap::new();

    // Here be dragons.

    // Basically, the first thing we do is populate files_by_imprint with any *potential* conflicts
    // (as determined by file length) from the base file list. The key for this process is the file
    // imprint, but the value is a struct called Conflict and all base file paths are inserted here
    // onto the conflict.base_files member. This process is performed for each path in the
    // comparison set.

    // We need this duplicate filter because we will get re-imprint base files for each file of
    // matching length in the comparison set, and this is... undesirable.
    let mut duplicate_filter = HashSet::new();

    for path in compare_files {
        let meta: Meta = path.metadata()?.into();
        if let Some(potential_conflicts) = base_files_by_length.get(&meta.len) {
            let imprints = potential_conflicts
                .iter()
                .copied()
                .filter(|&path| duplicate_filter.insert(path))
                .filter_map(|path| Imprint::new(path).ok().map(|imprint| (path, imprint)));

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
        if let EntryRef::Occupied(mut conflicts) = files_by_imprint.entry_ref(&imprint) {
            conflicts.get_mut().compare_files.push(path);
            context.cache.insert(path, meta);
        }
    }

    Ok(files_by_imprint
        .into_iter()
        .filter(|entry| !entry.1.compare_files.is_empty()))
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
                .copied()
                .and_then(|path| cache.get(path).map(|cx| cx.len))
                .unwrap_or_default();

        writeln!(
            handle,
            "{imprint}\n-------------------------- base files --------------------------",
        )?;

        for &path in &conflict.base_files {
            writeln!(handle, "{}", path.file_name().unwrap().to_string_lossy())?;
        }

        writeln!(
            handle,
            "-------------------------- duplicates --------------------------",
        )?;

        for &path in &conflict.compare_files {
            writeln!(handle, "{}", path.file_name().unwrap().to_string_lossy())?;
        }
        writeln!(handle)?;
    }

    writeln!(
        handle,
        "{count} duplicates ({})",
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
        let meta = fs::metadata(path)?;
        map.entry(meta.len()).or_insert_with(Vec::new).push(path);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use bumpalo::Bump;

    use crate::meta::Metacache;

    use super::{Context, get_conflicts};

    #[test]
    fn subtree_comparisons_ignore_subtree_files() {
        let paths = Bump::new();
        let mut context = Context {
            root: "./resource/test-folder/subfolder",
            compare_to: &["./resource/test-folder"],
            paths: &paths,
            cache: Metacache::new(),
            ignore: &[],
        };

        let actual: Vec<_> = get_conflicts(&mut context, true)
            .unwrap()
            .flat_map(|(_, conflict)| conflict.compare_files)
            .collect();
        let expected = &[Path::new("./resource/test-folder/a.txt")
            .canonicalize()
            .unwrap()];

        assert_eq!(actual, expected);
    }

    #[test]
    fn subtree_comparisons_do_not_result_in_duplicate_base_files() {
        let paths = Bump::new();
        let mut context = Context {
            root: "./resource/test-folder/subfolder",
            compare_to: &["./resource/test-folder"],
            paths: &paths,
            cache: Metacache::new(),
            ignore: &[],
        };

        let actual: Vec<_> = get_conflicts(&mut context, true)
            .unwrap()
            .flat_map(|(_, conflict)| conflict.base_files)
            .collect();
        let expected = &[
            Path::new("./resource/test-folder/subfolder/sub-a.txt")
                .canonicalize()
                .unwrap(),
            Path::new("./resource/test-folder/subfolder/sub-a-copy.txt")
                .canonicalize()
                .unwrap(),
        ];

        assert_eq!(actual, expected);
    }
}
