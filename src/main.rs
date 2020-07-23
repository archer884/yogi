mod format;
mod multiple;
mod opt;
mod rank;
mod single;

use imprint::Imprint;
use opt::Opt;
use std::path::Path;
use std::time::SystemTime;
use std::{fs, io};
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

fn main() -> io::Result<()> {
    let opt = Opt::from_args();
    if opt.compare.is_empty() {
        single::process(opt.path(), opt.sort_order(), opt.force)
    } else {
        multiple::process(opt.path(), &opt.compare, opt.force)
    }
}

fn list_entries(root: impl AsRef<Path>) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
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
    use format::{ByteSize, HexFormatter};
    use std::io::Write;

    let handle = io::stdout();
    let mut handle = handle.lock();
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
            "{:x}\n================================================================",
            HexFormatter(&imprint.head)
        )?;

        for path in group {
            writeln!(handle, "{}", path.display())?;
        }
        writeln!(handle)?;
    }

    writeln!(handle, "{} duplicates ({})", count, size.bytes(),)?;
    Ok(())
}
