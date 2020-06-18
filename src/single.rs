use crate::{format::BytesFormatter, rank::PathRanker};
use bumpalo::Bump;
use hashbrown::HashMap;
use imprint::Imprint;
use std::cmp::Reverse;
use std::io;
use std::path::Path;

pub fn process(path: &str, force: bool) -> io::Result<()> {
    // Do not reorder these two variables, because it will cause stupidly confusing lifetime
    // errors to appear.
    let paths = Bump::new();
    let mut metacache = HashMap::new();

    let ranker = PathRanker::new();
    let conflicts_by_len = build_conflicts_by_length(path, &paths, &mut metacache)?;
    let mut conflicts_by_imprint = build_conflicts_by_imprint(conflicts_by_len)?;

    // Sorting before deconfliction or formatting ensures that deconfliction behavior is
    // previewed appropriately.
    conflicts_by_imprint
        .iter_mut()
        .for_each(|x| x.1.sort_by_cached_key(|&x| Reverse(ranker.rank(x))));

    if force {
        let (count, size) = super::deconflict(conflicts_by_imprint, &metacache)?;
        println!("Removed {} files ({})", count, BytesFormatter::new(size));
    } else {
        super::pretty_print_conflicts(conflicts_by_imprint, &metacache)?;
    }

    Ok(())
}

fn build_conflicts_by_length<'a>(
    path: &str,
    path_src: &'a Bump,
    metacache: &mut HashMap<&'a Path, u64>,
) -> io::Result<impl Iterator<Item = &'a Path>> {
    let mut candidates = HashMap::new();

    for entry in super::list_entries(path) {
        let path = &**path_src.alloc(entry.path().to_owned());
        let len = path.metadata()?.len() as u64;
        metacache.insert(path, len);
        candidates.entry(len).or_insert_with(Vec::new).push(path);
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
