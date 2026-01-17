//! f — File Manager core library
//!
//! This crate exposes a small filesystem utility API used by the `f` binary GUI.
//! The API is intentionally minimal to be suitable for publishing on crates.io.
//!
//! # Example
//!
//! ```rust
//! use std::path::Path;
//! let entries = f::fs::list_dir_names(Path::new("."))?;
//! for e in entries { println!("{}", e); }
//! # Ok::<(), anyhow::Error>(())
//! ```

use anyhow::Result;
use std::time::SystemTime;

/// A directory entry returned by the small API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The file or directory name (no parent path)
    pub name: String,
    /// Whether this entry is a directory
    pub is_dir: bool,
    /// File size in bytes when applicable
    pub size: Option<u64>,
    /// Last modification timestamp
    pub modified: Option<SystemTime>,
}

/// Filesystem helper functions used by the GUI and suitable for reuse.
pub mod fs {
    use super::{Entry, Result};
    use anyhow::anyhow;
    use fs_extra::dir::{self, CopyOptions};
    use std::cmp::Ordering;
    use std::fs;
    use std::path::Path;

    /// List directory entries as `Entry` structs.
    ///
    /// Returns an empty Vec when the directory cannot be read.
    pub fn list_dir(path: &Path) -> Result<Vec<Entry>> {
        let mut entries = Vec::new();
        if let Ok(read) = fs::read_dir(path) {
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let metadata = entry.metadata().ok();
                let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = metadata.as_ref().filter(|m| m.is_file()).map(|m| m.len());
                let modified = metadata.and_then(|m| m.modified().ok());
                entries.push(Entry {
                    name,
                    is_dir,
                    size,
                    modified,
                });
            }
        }
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
        Ok(entries)
    }

    /// Convenience helper to return `Vec<String>` of names (directories end with `/`).
    pub fn list_dir_names(path: &Path) -> Result<Vec<String>> {
        let list = list_dir(path)?;
        Ok(list
            .into_iter()
            .map(|e| {
                if e.is_dir {
                    format!("{}/", e.name)
                } else {
                    e.name
                }
            })
            .collect())
    }

    /// Copy a file or directory into `dest_dir` (dest_dir must be a directory).
    /// For files the destination path will be `dest_dir/<file_name>`.
    pub fn copy_to(src: &Path, dest_dir: &Path) -> Result<()> {
        if src.is_dir() {
            let mut opts = CopyOptions::new();
            opts.copy_inside = true;
            let _ = dir::copy(src, dest_dir, &opts)?;
        } else {
            if let Some(name) = src.file_name() {
                let dest = dest_dir.join(name);
                let _ = fs::copy(src, &dest)?;
            }
        }
        Ok(())
    }

    /// Move a file or directory into `dest_dir` (similar semantics as copy_to).
    pub fn move_to(src: &Path, dest_dir: &Path) -> Result<()> {
        let Some(name) = src.file_name() else {
            return Err(anyhow!("Source path has no file name"));
        };
        let dest = dest_dir.join(name);
        std::fs::rename(src, dest)?;
        Ok(())
    }

    /// Delete a file or directory (recursively for directories).
    pub fn delete(path: &Path) -> Result<()> {
        if path.is_dir() {
            let _ = fs::remove_dir_all(path)?;
        } else {
            let _ = fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Create a directory at the provided path.
    pub fn create_dir(path: &Path) -> Result<()> {
        fs::create_dir(path)?;
        Ok(())
    }

    /// Search for entries in the given directory whose name contains `pattern`.
    pub fn search(path: &Path, pattern: &str) -> Result<Vec<Entry>> {
        let all = list_dir(path)?;
        Ok(all
            .into_iter()
            .filter(|e| e.name.contains(pattern))
            .collect())
    }

    /// Read a UTF-8 text file fully into memory.
    pub fn read_text(path: &Path) -> Result<String> {
        fs::read_to_string(path).map_err(Into::into)
    }

    /// Rename a path inside its parent directory, validating conflicts.
    pub fn rename_within_parent(src: &Path, new_name: &str) -> Result<()> {
        let Some(parent) = src.parent() else {
            return Err(anyhow!("Invalid path"));
        };
        let dest = parent.join(new_name);
        if dest.exists() {
            return Err(anyhow!("Destination already exists"));
        }
        fs::rename(src, dest)?;
        Ok(())
    }
}

/// Hashing helpers for generating common digests on files.
pub mod hash {
    use super::Result;
    use crc32fast::Hasher as Crc32Hasher;
    use md5::Context as Md5Context;
    use sha1::Sha1;
    use sha2::Sha256;
    use sha3::{digest::Digest, Sha3_256};
    use std::fmt::Write;
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    /// Collection of frequently requested hashes for a file.
    #[derive(Debug, Clone)]
    pub struct FileHashes {
        pub md5: String,
        pub crc32: String,
        pub sha1: String,
        pub sha256: String,
        pub sha3_256: String,
    }

    impl FileHashes {
        /// Render the hashes into a human-readable report for a path.
        pub fn to_report(&self, path: &Path) -> String {
            format!(
                "File: {path}\nMD5     : {md5}\nCRC32   : {crc32}\nSHA1    : {sha1}\nSHA256  : {sha256}\nSHA3-256: {sha3}\n",
                path = path.display(),
                md5 = self.md5,
                crc32 = self.crc32,
                sha1 = self.sha1,
                sha256 = self.sha256,
                sha3 = self.sha3_256,
            )
        }
    }

    /// Compute MD5, CRC32, SHA1, SHA256, and SHA3-256 for the given file.
    pub fn compute(path: &Path) -> Result<FileHashes> {
        let mut file = File::open(path)?;
        let mut buffer = [0u8; 16 * 1024];
        let mut md5_ctx = Md5Context::new();
        let mut sha1_ctx = Sha1::new();
        let mut sha256_ctx = Sha256::new();
        let mut sha3_ctx = Sha3_256::new();
        let mut crc_ctx = Crc32Hasher::new();

        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            let chunk = &buffer[..read];
            md5_ctx.consume(chunk);
            sha1_ctx.update(chunk);
            sha256_ctx.update(chunk);
            sha3_ctx.update(chunk);
            crc_ctx.update(chunk);
        }

        let md5_hex = bytes_to_hex(md5_ctx.compute().as_ref());
        let sha1_hex = bytes_to_hex(sha1_ctx.finalize().as_ref());
        let sha256_hex = bytes_to_hex(sha256_ctx.finalize().as_slice());
        let sha3_hex = bytes_to_hex(sha3_ctx.finalize().as_slice());
        let crc_hex = format!("{:08x}", crc_ctx.finalize());

        Ok(FileHashes {
            md5: md5_hex,
            crc32: crc_hex,
            sha1: sha1_hex,
            sha256: sha256_hex,
            sha3_256: sha3_hex,
        })
    }

    fn bytes_to_hex(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            let _ = write!(&mut output, "{:02x}", byte);
        }
        output
    }
}

/// Simple ZIP helper for compressing a single file or directory tree.
pub mod archive {
    use super::Result;
    use anyhow::anyhow;
    use std::fs::File;
    use std::io;
    use std::path::Path;
    use walkdir::WalkDir;
    use zip::write::FileOptions;
    use zip::{CompressionMethod, ZipWriter};

    /// Create a ZIP archive at `dest` containing `src`.
    pub fn zip_path(src: &Path, dest: &Path) -> Result<()> {
        let file = File::create(dest)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

        if src.is_file() {
            let name = src
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("archive");
            zip.start_file(name, options)?;
            let mut input = File::open(src)?;
            io::copy(&mut input, &mut zip)?;
        } else {
            let parent = src.parent().ok_or_else(|| anyhow!("Invalid source"))?;
            for entry in WalkDir::new(src) {
                let entry = entry?;
                let path = entry.path();
                let relative = path
                    .strip_prefix(parent)
                    .map_err(|_| anyhow!("Failed to build archive entry"))?;
                let mut rel = relative.to_string_lossy().replace('\\', "/");
                if entry.file_type().is_dir() {
                    if !rel.ends_with('/') {
                        rel.push('/');
                    }
                    zip.add_directory(&rel, options)?;
                } else {
                    zip.start_file(&rel, options)?;
                    let mut input = File::open(path)?;
                    io::copy(&mut input, &mut zip)?;
                }
            }
        }

        zip.finish()?;
        Ok(())
    }
}

/// Offline registration helpers shared by the GUI and CLI tools.
pub mod registration {
    use super::Result;
    use anyhow::{anyhow, bail};
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use sha2::{Digest, Sha256};

    pub const CODE_PREFIX: &str = "FREG1";
    const SECRET: &[u8; 32] = b"f::cmd::reg::seed-2025-guard-key";
    const SIGNATURE_LEN: usize = 12;

    /// Minimal registration data returned after successful verification.
    #[derive(Debug, Clone)]
    pub struct License {
        pub name: String,
    }

    /// Verify a registration code and obtain the associated license info.
    pub fn verify_code(code: &str) -> Result<License> {
        let trimmed = code.trim();
        let Some(payload) = trimmed.strip_prefix(&format!("{CODE_PREFIX}-")) else {
            bail!("Invalid code prefix");
        };
        let decoded = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| anyhow!("Invalid code encoding"))?;
        let payload = String::from_utf8(decoded).map_err(|_| anyhow!("Invalid code body"))?;
        let mut parts = payload.splitn(2, '|');
        let name = parts.next().unwrap_or("");
        let sig_hex = parts.next().unwrap_or("");
        if sig_hex.len() != SIGNATURE_LEN * 2 {
            bail!("Invalid signature length");
        }
        let clean = sanitize_name(name)?;
        let expected = proof_for(&clean);
        let provided = decode_hex(sig_hex)?;
        if provided != expected {
            bail!("Signature mismatch");
        }
        Ok(License { name: clean })
    }

    fn sanitize_name(input: &str) -> Result<String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            bail!("Registration name cannot be empty");
        }
        if trimmed.len() > 64 {
            bail!("Registration name too long");
        }
        Ok(trimmed.replace(['\r', '\n'], " "))
    }

    fn proof_for(name: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.update(SECRET);
        let digest = hasher.finalize();
        digest[..SIGNATURE_LEN].to_vec()
    }

    fn decode_hex(input: &str) -> Result<Vec<u8>> {
        if input.len() % 2 != 0 {
            bail!("Invalid hex payload");
        }
        let mut bytes = Vec::with_capacity(input.len() / 2);
        let chars: Vec<char> = input.chars().collect();
        for chunk in chars.chunks(2) {
            let hi = from_hex(chunk[0])?;
            let lo = from_hex(chunk[1])?;
            bytes.push((hi << 4) | lo);
        }
        Ok(bytes)
    }

    fn from_hex(ch: char) -> Result<u8> {
        match ch {
            '0'..='9' => Ok(ch as u8 - b'0'),
            'a'..='f' => Ok(ch as u8 - b'a' + 10),
            'A'..='F' => Ok(ch as u8 - b'A' + 10),
            _ => bail!("Invalid hex character"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fs;
    use anyhow::Result;
    use std::fs::{File, create_dir_all};
    use std::path::PathBuf;

    #[test]
    fn list_and_search() -> Result<()> {
        let td = tempfile::tempdir()?;
        let base = td.path().join("a_dir");
        create_dir_all(&base)?;
        File::create(base.join("file1.txt"))?;
        File::create(base.join("file2.log"))?;
        let names = fs::list_dir_names(&base)?;
        assert!(names.contains(&"file1.txt".to_string()));
        assert!(names.contains(&"file2.log".to_string()));
        let search = fs::search(&base, "file1")?;
        assert_eq!(search.len(), 1);
        Ok(())
    }

    #[test]
    fn copy_and_delete_file() -> Result<()> {
        let td = tempfile::tempdir()?;
        let base = td.path().join("base");
        let dest = td.path().join("dest");
        create_dir_all(&base)?;
        create_dir_all(&dest)?;
        File::create(base.join("x.txt"))?;
        fs::copy_to(&base.join("x.txt"), &dest)?;
        let names = fs::list_dir_names(&dest)?;
        assert!(names.contains(&"x.txt".to_string()));
        fs::delete(&dest.join("x.txt"))?;
        Ok(())
    }
}
