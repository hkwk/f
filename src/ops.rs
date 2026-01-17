use std::path::PathBuf;

use iced::Task;

use crate::messages::Message;
use crate::model::Side;
use crate::types::{FileHashes, FsResult};
use anyhow::Error;
use f::{archive, hash, Entry, fs as fs_utils};

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
    fs_utils::read_text(&path).map_err(|err: Error| err.to_string())
}

pub async fn rename_path(src: PathBuf, new_name: String) -> FsResult<()> {
    fs_utils::rename_within_parent(&src, &new_name).map_err(|err: Error| err.to_string())
}

pub async fn compress_path(src: PathBuf, dest: PathBuf) -> FsResult<()> {
    archive::zip_path(&src, &dest).map_err(|err: Error| err.to_string())
}

pub async fn compute_hashes(path: PathBuf) -> FsResult<FileHashes> {
    hash::compute(&path).map_err(|err: Error| err.to_string())
}
