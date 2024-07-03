use std::{cmp::Reverse, io, path::Path};

use bumpalo::Bump;
use fmtsize::{Conventional, FmtSize};
use hashbrown::HashMap;
use imprint::Imprint;

use crate::{config::SortOrder, meta::Meta, rank::PathRanker, Metacache};

trait PathSorter {
    fn sort(&self, paths: &mut [&Path]);
}

impl PathSorter for PathRanker {
    fn sort(&self, paths: &mut [&Path]) {
        paths.sort_by_cached_key(|&p| Reverse(self.rank(p)));
    }
}

struct MetaSorter<'a> {
    by_newest: bool,
    cache: &'a Metacache<'a>,
}

impl PathSorter for MetaSorter<'_> {
    fn sort(&self, paths: &mut [&Path]) {
        if self.by_newest {
            paths.sort_by_key(|&path| Reverse(self.cache.get(path).unwrap().created));
        } else {
            paths.sort_by_key(|&path| self.cache.get(path).unwrap().created);
        }
    }
}

fn get_sorter<'a>(sort: SortOrder, cache: &'a Metacache<'a>) -> Box<dyn PathSorter + 'a> {
    match sort {
        SortOrder::Descriptive => Box::new(PathRanker::new()),
        SortOrder::Newest => Box::new(MetaSorter {
            by_newest: true,
            cache,
        }),
        SortOrder::Oldest => Box::new(MetaSorter {
            by_newest: false,
            cache,
        }),
    }
}

pub fn process(path: &str, sort: SortOrder, force: bool, recurse: bool) -> io::Result<()> {
    // We need the arena to be allocated first so that it can be dropped last.
    let paths = Bump::new();

    let mut metacache = Metacache::new();

    let conflicts_by_len = build_conflicts_by_length(path, &paths, &mut metacache, recurse)?;
    let mut conflicts_by_imprint = build_conflicts_by_imprint(conflicts_by_len)?;

    // Sorting before deconfliction or formatting ensures that deconfliction behavior is
    // previewed appropriately.
    let sorter = get_sorter(sort, &metacache);
    conflicts_by_imprint
        .iter_mut()
        .for_each(|x| sorter.sort(&mut x.1));

    if force {
        let (count, size) = super::deconflict(conflicts_by_imprint, &metacache)?;
        println!("Removed {} files ({})", count, size.fmt_size(Conventional));
    } else {
        super::pretty_print_conflicts(conflicts_by_imprint, &metacache)?;
    }

    Ok(())
}

fn build_conflicts_by_length<'a>(
    path: &str,
    path_src: &'a Bump,
    metacache: &mut Metacache<'a>,
    recurse: bool,
) -> io::Result<impl Iterator<Item = &'a Path>> {
    let mut candidates = HashMap::new();

    for entry in super::list_entries(path, recurse) {
        let path = &**path_src.alloc(entry.path().to_owned());
        let meta: Meta = path.metadata()?.into();
        candidates
            .entry(meta.len)
            .or_insert_with(Vec::new)
            .push(path);
        metacache.insert(path, meta);
    }

    Ok(candidates
        .into_iter()
        .filter(|x| x.1.len() > 1)
        .flat_map(|x| x.1.into_iter()))
}

fn build_conflicts_by_imprint<'a>(
    paths: impl IntoIterator<Item = &'a Path>,
) -> io::Result<Vec<(Imprint, Vec<&'a Path>)>> {
    let mut candidates = HashMap::new();

    for path in paths {
        let imprint = Imprint::new(path)?;
        candidates
            .entry(imprint)
            .or_insert_with(Vec::new)
            .push(path);
    }

    Ok(candidates.into_iter().filter(|x| x.1.len() > 1).collect())
}
