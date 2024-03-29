use std::{fs, io, path::Path, time::SystemTime};

mod multiple;
mod opt;
mod rank;
mod single;

use imprint::Imprint;
use opt::Opts;
use walkdir::{DirEntry, WalkDir};

type Metacache<'a> = hashbrown::HashMap<&'a Path, Meta>;

#[derive(Clone, Debug)]
struct Meta {
    // The only time I've ever seen this fail was in pulling metadata for files on a Windows
    // volume from a Linux host. Whether it can happen under any other circumstances, God knows.
    // Hopefully God also knows what happens if you prioritize files by created date and they
    // don't freaking have one.
    created: Option<SystemTime>,
    len: u64,
}

impl From<fs::Metadata> for Meta {
    fn from(meta: fs::Metadata) -> Self {
        Self {
            created: meta.created().ok(),
            len: meta.len(),
        }
    }
}

fn main() {
    if let Err(e) = run(&Opts::parse()) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run(opts: &Opts) -> io::Result<()> {
    if opts.compare.is_empty() {
        single::process(opts.path(), opts.sort_order(), opts.force, opts.recurse())
    } else {
        multiple::process(opts.path(), &opts.compare, opts.force, opts.recurse())
    }
}

fn list_entries(root: impl AsRef<Path>, recurse: bool) -> impl Iterator<Item = DirEntry> {
    fn is_file(entry: &DirEntry) -> bool {
        entry.file_type().is_file()
    }

    if recurse {
        WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(is_file)
    } else {
        WalkDir::new(root)
            .max_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(is_file)
    }
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
