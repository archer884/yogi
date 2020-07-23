use std::fmt::{self, Display, LowerHex};

const KILOBYTE: u64 = 1 << 10;
const MEGABYTE: u64 = 1 << 20;
const GIGABYTE: u64 = 1 << 30;

pub trait ByteSize {
    fn bytes(self) -> ByteSizeFormatter;
}

impl ByteSize for u64 {
    fn bytes(self) -> ByteSizeFormatter {
        match self {
            size if size < MEGABYTE => ByteSizeFormatter::Kilobytes(size as f64 / KILOBYTE as f64),
            size if size < GIGABYTE => ByteSizeFormatter::Megabytes(size as f64 / MEGABYTE as f64),
            size => ByteSizeFormatter::Gigabytes(size as f64 / GIGABYTE as f64),
        }
    }
}

pub enum ByteSizeFormatter {
    Kilobytes(f64),
    Megabytes(f64),
    Gigabytes(f64),
}

impl Display for ByteSizeFormatter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ByteSizeFormatter::Kilobytes(size) => write!(f, "{:.2} KB", size),
            ByteSizeFormatter::Megabytes(size) => write!(f, "{:.2} MB", size),
            ByteSizeFormatter::Gigabytes(size) => write!(f, "{:.2} GB", size),
        }
    }
}

pub struct HexFormatter<'a>(pub &'a [u8]);

impl LowerHex for HexFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.iter().try_for_each(|&u| write!(f, "{:02x}", u))
    }
}
