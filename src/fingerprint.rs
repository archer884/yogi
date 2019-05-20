use std::io::{self, Read};
use std::path::Path;

const MAX_SIZE: u64 = 0x0080_0000;

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct PartialFingerprint(u64);

impl PartialFingerprint {
    pub fn from_path<T: AsRef<Path>>(path: T) -> io::Result<Self> {
        use std::fs::File;
        let file = File::open(path)?;
        let meta = file.metadata()?;
        Ok(PartialFingerprint(meta.len()))
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Fingerprint {
    length: u64,
    head: Vec<u8>, // Here we use a vec solely because an array lacks eq and hash definitions.
    tail: Option<Vec<u8>>,
}

impl Fingerprint {
    pub fn from_path<T: AsRef<Path>>(path: T) -> io::Result<Self> {
        use std::cmp;
        use std::fs::File;
        use std::io::Seek;
        use std::io::SeekFrom;

        let mut file = File::open(path)?;
        let meta = file.metadata()?;

        let head = hash(&mut file)?;
        let tail = {
            if meta.len() > MAX_SIZE {
                let extra = if MAX_SIZE > meta.len() {
                    cmp::min(MAX_SIZE, meta.len() - MAX_SIZE)
                } else {
                    MAX_SIZE
                };

                file.seek(SeekFrom::End(extra as i64))?;
                Some(hash(&mut file)?)
            } else {
                None
            }
        };

        Ok(Fingerprint {
            length: meta.len(),
            head,
            tail,
        })
    }
}

fn hash(mut stream: impl Read) -> io::Result<Vec<u8>> {
    use sha2::Sha256;
    use digest::{FixedOutput, Input};

    // We need box emplacement here because, otherwise, it blows the stack.
    let mut buf = box [0u8; MAX_SIZE as usize];
    let len = stream.read(&mut *buf)?;

    let mut hasher = Sha256::default();
    hasher.process(&buf[..len]);

    Ok(hasher.fixed_result().into_iter().collect())
}
