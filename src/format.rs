use std::fmt::{self, Display, LowerHex};

const KILOBYTE: u64 = 1 << 10;
const MEGABYTE: u64 = 1 << 20;
const GIGABYTE: u64 = 1 << 30;

pub enum BytesFormatter {
    Kilobytes(f64),
    Megabytes(f64),
    Gigabytes(f64),
}

impl BytesFormatter {
    pub fn new(size: u64) -> Self {
        match size {
            size if size < MEGABYTE => BytesFormatter::Kilobytes(size as f64 / KILOBYTE as f64),
            size if size < GIGABYTE => BytesFormatter::Megabytes(size as f64 / MEGABYTE as f64),
            size => BytesFormatter::Gigabytes(size as f64 / GIGABYTE as f64),
        }
    }
}

impl Display for BytesFormatter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BytesFormatter::Kilobytes(size) => write!(f, "{:.2} KB", size),
            BytesFormatter::Megabytes(size) => write!(f, "{:.2} MB", size),
            BytesFormatter::Gigabytes(size) => write!(f, "{:.2} GB", size),
        }
    }
}

pub struct HexFormatter<'a>(pub &'a [u8]);

impl LowerHex for HexFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.iter().try_for_each(|&u| write!(f, "{:02x}", u))
    }
}
