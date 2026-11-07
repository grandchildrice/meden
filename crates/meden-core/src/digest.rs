//! Content digests for artifacts.
//!
//! A skill package is a directory. Its digest is the SHA-256 of the
//! concatenation, for every regular file in sorted relative-path order, of
//! `path-length(8 bytes, little endian) || path(utf-8) || size(8 bytes, little
//! endian) || contents`. Directories, symlinks and anything that is not a
//! regular file are ignored. Two trees with the same files and contents have
//! the same digest wherever they sit on disk.

use sha2::{Digest, Sha256};

/// Hex SHA-256 of a byte string.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Incremental digest over `(relative path, contents)` entries.
///
/// Callers must feed entries in ascending order of their path bytes; the
/// builder does not sort. [`DigestBuilder::finish`] returns the hex digest.
#[derive(Default)]
pub struct DigestBuilder {
    hasher: Sha256,
}

impl DigestBuilder {
    /// A fresh builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one file.
    pub fn file(&mut self, relative_path: &str, contents: &[u8]) {
        let path = relative_path.as_bytes();
        self.hasher.update((path.len() as u64).to_le_bytes());
        self.hasher.update(path);
        self.hasher.update((contents.len() as u64).to_le_bytes());
        self.hasher.update(contents);
    }

    /// Hex digest.
    pub fn finish(self) -> String {
        hex::encode(self.hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_order_and_boundary_sensitive() {
        let mut a = DigestBuilder::new();
        a.file("SKILL.md", b"ab");
        a.file("scripts/x.sh", b"c");
        let mut b = DigestBuilder::new();
        b.file("SKILL.md", b"a");
        b.file("scripts/x.sh", b"bc");
        assert_ne!(a.finish(), b.finish());
        assert_eq!(sha256_hex(b"").len(), 64);
    }
}
