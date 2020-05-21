use structopt::StructOpt;

/// Examine a directory for duplicated files and remove them.
#[derive(Clone, Debug, StructOpt)]
pub struct Opt {
    /// The root path to be examined
    ///
    /// Defaults to "." if no value is provided.
    path: Option<String>,

    /// Additional paths (files in root path will be preferred)
    #[structopt(short, long)]
    pub compare: Vec<String>,

    /// Remove duplicate files.
    #[structopt(short = "f", long = "force")]
    pub force: bool,
}

impl Opt {
    pub fn from_args() -> Self {
        StructOpt::from_args()
    }

    pub fn path(&self) -> &str {
        self.path.as_ref().map(AsRef::as_ref).unwrap_or(".")
    }
}
