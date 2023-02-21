use std::{hash::Hash, path::PathBuf};

use hashbrown::{hash_map::RawEntryMut, HashMap};

pub struct CachedFileState {
    path: PathBuf,
    len: u64,
    hash: (),
}

pub struct Cache<T, U> {
    store: HashMap<T, U>,
}

impl<T, U> Cache<T, U>
where
    T: Hash + Eq + ToOwned<Owned = T>,
{
    fn get<'a>(&'a mut self, key: &T, mut f: impl FnMut(&T) -> U) -> &'a U {
        match self.store.raw_entry_mut().from_key(key) {
            RawEntryMut::Occupied(entry) => entry.into_mut(),
            RawEntryMut::Vacant(entry) => {
                let (_, value) = entry.insert(key.to_owned(), f(key));
                value
            }
        }
    }
}
