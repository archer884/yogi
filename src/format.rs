use std::fmt::{self, LowerHex};

pub struct HexFormatter<'a>(pub &'a [u8]);

impl LowerHex for HexFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.iter().try_for_each(|&u| write!(f, "{:02x}", u))
    }
}
