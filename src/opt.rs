use std::str::FromStr;

use structopt::StructOpt;

/// Examine a directory for duplicated files and remove them.
#[derive(Clone, Debug, StructOpt)]
pub struct Opt {
    /// The root path to be examined
    /// Defaults to "."
    path: Option<String>,

    /// Additional paths (files in root path will be preferred)
    #[structopt(short, long)]
    pub compare: Vec<String>,

    /// Remove duplicate files.
    #[structopt(short = "f", long = "force")]
    pub force: bool,

    /// Keep 'oldest' or 'newest' files instead of 'most descriptive.'
    ///
    /// Note that this only applies to the single tree process.
    #[structopt(short, long)]
    pub sort: Option<SortOrder>,

    /// Do not recurse into subdirectories (applies to root path)
    #[structopt(short, long)]
    pub no_recurse: bool,
}

#[derive(Copy, Clone, Debug)]
pub enum SortOrder {
    Descriptive,
    Newest,
    Oldest,
}

impl FromStr for SortOrder {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "d" | "descriptive" => Ok(SortOrder::Descriptive),
            "o" | "oldest" => Ok(SortOrder::Oldest),
            "n" | "newest" => Ok(SortOrder::Newest),

            _ => Err(format!(
                "{:?} is not a valid sort order; try oldest or newest",
                s
            )),
        }
    }
}

impl Opt {
    pub fn from_args() -> Self {
        StructOpt::from_args()
    }

    pub fn path(&self) -> &str {
        self.path.as_ref().map(AsRef::as_ref).unwrap_or(".")
    }

    pub fn sort_order(&self) -> SortOrder {
        self.sort.unwrap_or(SortOrder::Descriptive)
    }

    pub fn recurse(&self) -> bool {
        !self.no_recurse
    }
}
