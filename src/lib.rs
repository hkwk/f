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


/// A directory entry returned by the small API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The file or directory name (no parent path)
    pub name: String,
    /// Whether this entry is a directory
    pub is_dir: bool,
}

/// Filesystem helper functions used by the GUI and suitable for reuse.
pub mod fs {
    use super::{Entry, Result};
    use fs_extra::dir::{self, CopyOptions};
    use std::fs;
    use std::path::Path;

    /// List directory entries as `Entry` structs.
    ///
    /// Returns an empty Vec when the directory cannot be read.
    pub fn list_dir(path: &Path) -> Result<Vec<Entry>> {
        let mut entries = Vec::new();
        if let Ok(read) = fs::read_dir(path) {
            for entry in read.flatten() {
                if let Ok(ft) = entry.file_type() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    entries.push(Entry { name, is_dir: ft.is_dir() });
                }
            }
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    /// Convenience helper to return `Vec<String>` of names (directories end with `/`).
    pub fn list_dir_names(path: &Path) -> Result<Vec<String>> {
        let list = list_dir(path)?;
        Ok(list
            .into_iter()
            .map(|e| if e.is_dir { format!("{}/", e.name) } else { e.name })
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

    /// Delete a file or directory (recursively for directories).
    pub fn delete(path: &Path) -> Result<()> {
        if path.is_dir() {
            let _ = fs::remove_dir_all(path)?;
        } else {
            let _ = fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Search for entries in the given directory whose name contains `pattern`.
    pub fn search(path: &Path, pattern: &str) -> Result<Vec<Entry>> {
        let all = list_dir(path)?;
        Ok(all.into_iter().filter(|e| e.name.contains(pattern)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::fs;
    use anyhow::Result;
    use std::fs::{create_dir_all, File};
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
