use std::{ffi::OsStr, fs, io, path::Path};

mod config;
mod meta;
mod multiple;
mod rank;
mod single;

use config::Args;
use imprint::Imprint;
use meta::Metacache;
use walkdir::{DirEntry, WalkDir};

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run(args: Args) -> io::Result<()> {
    let ignore: Vec<_> = args.ignore.iter().map(OsStr::new).collect();

    if args.compare.is_empty() {
        single::process(
            args.path(),
            args.sort_order(),
            args.force,
            args.recurse(),
            &ignore,
        )
    } else {
        multiple::process(
            args.path(),
            &args.compare,
            args.force,
            args.recurse(),
            &ignore,
        )
    }
}

fn list_entries<'a>(
    root: impl AsRef<Path>,
    recurse: bool,
    ignore: &'a [&OsStr],
) -> impl Iterator<Item = DirEntry> + 'a {
    fn is_hidden(path: &Path) -> bool {
        let Some(file_name) = path.file_name() else {
            return false;
        };

        file_name
            .as_encoded_bytes()
            .starts_with(OsStr::new(".").as_encoded_bytes())
    }

    fn is_file(entry: &DirEntry) -> bool {
        entry.file_type().is_file() && entry.path().ancestors().all(|path| !is_hidden(path))
    }

    fn is_ignored(entry: &DirEntry, ignore: &[&OsStr]) -> bool {
        let Some(extension) = entry.path().extension() else {
            return false;
        };

        ignore.iter().copied().any(|i| i == extension)
    }

    let walker = if recurse {
        WalkDir::new(root).into_iter()
    } else {
        WalkDir::new(root).max_depth(1).into_iter()
    };

    walker
        .filter_map(Result::ok)
        .filter(is_file)
        .filter(move |entry| !is_ignored(entry, ignore))
}

fn deconflict<'a>(
    groups: impl IntoIterator<Item = (Imprint, Vec<&'a Path>)>,
    cache: &Metacache,
) -> io::Result<(usize, u64)> {
    let mut count = 0;
    let mut size = 0;

    let conflicts = groups.into_iter().flat_map(|x| x.1.into_iter().skip(1));
    for path in conflicts {
        count += 1;
        size += cache.get(path).map(|x| x.len).unwrap_or_default();
        fs::remove_file(path)?;
    }

    Ok((count, size))
}

fn pretty_print_conflicts<'a>(
    groups: impl IntoIterator<Item = (Imprint, Vec<&'a Path>)>,
    metacache: &Metacache,
) -> io::Result<()> {
    use fmtsize::{Conventional, FmtSize};
    use std::io::Write;

    let mut handle = io::stdout().lock();
    let mut count = 0;
    let mut size = 0;

    for (imprint, group) in groups {
        // One of these files is NOT a duplicate; it's the primary.
        let n = group.len() - 1;

        count += n;
        size += n as u64
            * group
                .first()
                .and_then(|&path| metacache.get(path).map(|x| x.len))
                .unwrap_or_default();

        writeln!(
            handle,
            "{}\n================================================================",
            imprint,
        )?;

        for path in group {
            writeln!(handle, "{}", path.display())?;
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
