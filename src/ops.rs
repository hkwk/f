use std::fmt::Write;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use iced::Task;
use md5::Context as Md5Context;
use sha1::Sha1;
use sha3::Sha3_256;
use sha3::digest::Digest;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::messages::Message;
use crate::model::Side;
use crate::types::{FileHashes, FsResult};
use anyhow::Error;
use crc32fast::Hasher as Crc32Hasher;
use sha2::Sha256;
use f::{Entry, fs as fs_utils};

pub fn load_panel(side: Side, path: PathBuf) -> Task<Message> {
    Task::perform(list_entries(path.clone()), move |result| match result {
        Ok(entries) => Message::PanelLoaded {
            side,
            path,
            entries,
        },
        Err(error) => Message::PanelLoadFailed { side, path, error },
    })
}

pub async fn list_entries(path: PathBuf) -> FsResult<Vec<Entry>> {
    fs_utils::list_dir(&path).map_err(|err: Error| err.to_string())
}

pub async fn copy_entry(src: PathBuf, dest_dir: PathBuf) -> FsResult<()> {
    fs_utils::copy_to(&src, &dest_dir).map_err(|err: Error| err.to_string())
}

pub async fn move_entry(src: PathBuf, dest_dir: PathBuf) -> FsResult<()> {
    fs_utils::move_to(&src, &dest_dir).map_err(|err: Error| err.to_string())
}

pub async fn delete_entry(target: PathBuf) -> FsResult<()> {
    fs_utils::delete(&target).map_err(|err: Error| err.to_string())
}

pub async fn create_directory(path: PathBuf) -> FsResult<()> {
    fs_utils::create_dir(&path).map_err(|err: Error| err.to_string())
}

pub async fn read_text_file(path: PathBuf) -> FsResult<String> {
    fs::read_to_string(&path).map_err(|err| err.to_string())
}

pub async fn rename_path(src: PathBuf, new_name: String) -> FsResult<()> {
    let Some(parent) = src.parent().map(|p| p.to_path_buf()) else {
        return Err(String::from("Invalid path"));
    };
    let dest = parent.join(&new_name);
    if dest.exists() {
        return Err(String::from("Destination already exists"));
    }
    fs::rename(&src, &dest).map_err(|err| err.to_string())
}

pub async fn compress_path(src: PathBuf, dest: PathBuf) -> FsResult<()> {
    let file = fs::File::create(&dest).map_err(|err| err.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

    if src.is_file() {
        let name = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("archive");
        zip.start_file(name, options)
            .map_err(|err| err.to_string())?;
        let mut input = fs::File::open(&src).map_err(|err| err.to_string())?;
        io::copy(&mut input, &mut zip).map_err(|err| err.to_string())?;
    } else {
        let Some(parent) = src.parent().map(|p| p.to_path_buf()) else {
            return Err(String::from("Invalid source"));
        };
        for entry in WalkDir::new(&src) {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            let relative = path
                .strip_prefix(&parent)
                .map_err(|_| String::from("Failed to build archive entry"))?;
            let mut rel = relative.to_string_lossy().replace('\\', "/");
            if entry.file_type().is_dir() {
                if !rel.ends_with('/') {
                    rel.push('/');
                }
                zip.add_directory(&rel, options)
                    .map_err(|err| err.to_string())?;
            } else {
                zip.start_file(&rel, options)
                    .map_err(|err| err.to_string())?;
                let mut input = fs::File::open(path).map_err(|err| err.to_string())?;
                io::copy(&mut input, &mut zip).map_err(|err| err.to_string())?;
            }
        }
    }

    zip.finish().map_err(|err| err.to_string())?;
    Ok(())
}

pub async fn compute_hashes(path: PathBuf) -> FsResult<FileHashes> {
    let mut file = fs::File::open(&path).map_err(|err| err.to_string())?;
    let mut buffer = [0u8; 16 * 1024];
    let mut md5_ctx = Md5Context::new();
    let mut sha1_ctx = Sha1::new();
    let mut sha256_ctx = Sha256::new();
    let mut sha3_ctx = Sha3_256::new();
    let mut crc_ctx = Crc32Hasher::new();

    loop {
        let read = file.read(&mut buffer).map_err(|err| err.to_string())?;
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

    let md5_bytes = md5_ctx.compute();
    let sha1_bytes = sha1_ctx.finalize();
    let sha256_bytes = sha256_ctx.finalize();
    let sha3_bytes = sha3_ctx.finalize();

    let md5_hex = bytes_to_hex(md5_bytes.as_ref());
    let sha1_hex = bytes_to_hex(&sha1_bytes);
    let sha256_hex = bytes_to_hex(sha256_bytes.as_slice());
    let sha3_hex = bytes_to_hex(sha3_bytes.as_slice());
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
