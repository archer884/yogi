use std::{fs, path::Path, time::SystemTime};

pub type Metacache<'a> = hashbrown::HashMap<&'a Path, Meta>;

#[derive(Clone, Debug)]
pub struct Meta {
    // The only time I've ever seen this fail was in pulling metadata for files on a Windows
    // volume from a Linux host. Whether it can happen under any other circumstances, God knows.
    // Hopefully God also knows what happens if you prioritize files by created date and they
    // don't freaking have one.
    pub created: Option<SystemTime>,
    pub len: u64,
}

impl From<fs::Metadata> for Meta {
    fn from(meta: fs::Metadata) -> Self {
        Self {
            created: meta.created().ok(),
            len: meta.len(),
        }
    }
}
