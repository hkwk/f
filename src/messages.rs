use std::path::PathBuf;

use crate::i18n::Language;
use crate::model::{Side, SortColumn};
use crate::types::{FileHashes, FsResult};
use f::Entry;
use iced::keyboard::{Key, Modifiers};

#[derive(Debug, Clone)]
pub enum Message {
    PanelLoaded {
        side: Side,
        path: PathBuf,
        entries: Vec<Entry>,
    },
    PanelLoadFailed {
        side: Side,
        path: PathBuf,
        error: String,
    },
    SearchChanged(Side, String),
    PathChanged(Side, String),
    PathSubmitted(Side),
    SelectEntry(Side, usize),
    SelectParent(Side),
    ToggleSort(Side, SortColumn),
    Refresh(Side),
    RefreshAll,
    NavigateUp(Side),
    Copy(Side),
    Move(Side),
    Delete(Side),
    NewFolderNameChanged(Side, String),
    CreateFolder(Side),
    QuickCreateFolder(Side),
    OperationFinished {
        kind: OperationKind,
        result: FsResult<()>,
    },
    StartRename(Side),
    RenameInputChanged(Side, String),
    SubmitRename(Side),
    CancelRename(Side),
    ViewFile(Side),
    ViewerLoaded {
        path: PathBuf,
        result: FsResult<String>,
    },
    CloseViewer,
    EditFile(Side),
    Compress(Side),
    SwapPanels,
    ComputeHashes(Side),
    HashesComputed {
        side: Side,
        path: PathBuf,
        result: FsResult<FileHashes>,
    },
    ToggleTheme,
    ToggleConfigPanel,
    ToggleAboutPanel,
    IncreaseFontSize,
    DecreaseFontSize,
    MoveSelection {
        side: Side,
        delta: i32,
    },
    LanguageSelected(Language),
    KeyboardShortcut {
        key: Key,
        modifiers: Modifiers,
    },
}

#[derive(Debug, Clone)]
pub enum OperationKind {
    Copy { from: Side, to: Side },
    Move { from: Side, to: Side },
    Delete { side: Side },
    CreateFolder { side: Side },
    Rename { side: Side },
    Compress { side: Side, dest: PathBuf },
}
