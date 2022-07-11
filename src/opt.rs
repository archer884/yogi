use std::{error::Error, fmt::Display, str::FromStr};

use clap::Parser;

/// Examine a directory for duplicated files and remove them.
#[derive(Clone, Debug, Parser)]
#[clap(version)]
pub struct Opts {
    /// The root path to be examined
    /// Defaults to "."
    path: Option<String>,

    /// Additional paths (files in root path will be preferred)
    #[clap(short, long)]
    pub compare: Vec<String>,

    /// Remove duplicate files.
    #[clap(short = 'f', long = "force")]
    pub force: bool,

    /// Keep 'oldest' or 'newest' files instead of 'most descriptive.'
    ///
    /// Note that this only applies to the single tree process.
    #[clap(short, long)]
    pub sort: Option<SortOrder>,

    /// Do not recurse into subdirectories (applies to root path)
    #[clap(short, long)]
    pub no_recurse: bool,
}

#[derive(Copy, Clone, Debug)]
pub enum SortOrder {
    Descriptive,
    Newest,
    Oldest,
}

impl FromStr for SortOrder {
    type Err = ParseSortOrderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "d" | "descriptive" => Ok(SortOrder::Descriptive),
            "o" | "oldest" => Ok(SortOrder::Oldest),
            "n" | "newest" => Ok(SortOrder::Newest),
            _ => Err(ParseSortOrderError(s.into())),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParseSortOrderError(String);

impl Display for ParseSortOrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} is not a valid sort order\nTry one of descriptive, oldest, newest",
            self.0
        )
    }
}

impl Error for ParseSortOrderError {}

impl Opts {
    pub fn parse() -> Self {
        Parser::parse()
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
