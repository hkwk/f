use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local};
use f::Entry;

use crate::i18n::{Language, Strings};

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PanelState {
    pub path: PathBuf,
    pub path_input: String,
    pub search: String,
    pub entries: Vec<DirEntryView>,
    pub selected: Option<usize>,
    pub status: PanelStatus,
    pub new_folder: String,
    pub rename_target: Option<usize>,
    pub rename_input: String,
    pub sort_column: SortColumn,
    pub sort_order: SortOrder,
    pub parent_selected: bool,
}

impl PanelState {
    pub fn new(path: PathBuf) -> Self {
        let path_input = path.display().to_string();
        Self {
            path,
            path_input,
            search: String::new(),
            entries: Vec::new(),
            selected: None,
            status: PanelStatus::Ready,
            new_folder: String::new(),
            rename_target: None,
            rename_input: String::new(),
            sort_column: SortColumn::Name,
            sort_order: SortOrder::Ascending,
            parent_selected: false,
        }
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path.clone();
        self.path_input = path.display().to_string();
        self.search.clear();
        self.selected = None;
        self.status = PanelStatus::Loading;
        self.new_folder.clear();
        self.rename_target = None;
        self.rename_input.clear();
        self.parent_selected = false;
    }

    pub fn can_go_up(&self) -> bool {
        self.path.parent().is_some()
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.parent_selected = false;
    }

    pub fn move_selection(&mut self, delta: i32) {
        let visible = self.visible_indexes();
        let has_parent = self.can_go_up();
        let total_rows = visible.len() + if has_parent { 1 } else { 0 };
        if total_rows == 0 {
            return;
        }

        let current_pos = if self.parent_selected {
            Some(0)
        } else if let Some(selected) = self.selected {
            visible
                .iter()
                .position(|idx| *idx == selected)
                .map(|pos| pos + if has_parent { 1 } else { 0 })
        } else {
            None
        };

        let mut new_pos = match current_pos {
            Some(pos) => pos as i32 + delta,
            None => {
                if delta >= 0 {
                    0
                } else {
                    total_rows as i32 - 1
                }
            }
        };
        new_pos = new_pos.clamp(0, total_rows as i32 - 1);

        if has_parent && new_pos == 0 {
            self.parent_selected = true;
            self.selected = None;
            return;
        }

        let entry_index = new_pos - if has_parent { 1 } else { 0 };
        if entry_index >= 0 {
            self.selected = visible.get(entry_index as usize).copied();
            self.parent_selected = false;
        }
    }

    pub fn selected_entry(&self) -> Option<&DirEntryView> {
        self.selected.and_then(|idx| self.entries.get(idx))
    }

    pub fn filtered_indexes(&self) -> Vec<usize> {
        if self.search.trim().is_empty() {
            return (0..self.entries.len()).collect();
        }
        let needle = self.search.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                if entry.name.to_lowercase().contains(&needle) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn visible_indexes(&self) -> Vec<usize> {
        let mut indexes = self.filtered_indexes();
        indexes.sort_by(|a, b| self.compare_for_sort(*a, *b));
        indexes
    }

    pub fn toggle_sort(&mut self, column: SortColumn) {
        if self.sort_column == column {
            self.sort_order = self.sort_order.toggled();
        } else {
            self.sort_column = column;
            self.sort_order = SortOrder::Ascending;
        }
    }

    fn compare_for_sort(&self, a_idx: usize, b_idx: usize) -> Ordering {
        let a_entry = &self.entries[a_idx];
        let b_entry = &self.entries[b_idx];

        if a_entry.is_dir && !b_entry.is_dir {
            return Ordering::Less;
        }
        if !a_entry.is_dir && b_entry.is_dir {
            return Ordering::Greater;
        }
        if a_entry.is_dir && b_entry.is_dir {
            return cmp_insensitive(a_entry.name_label(), b_entry.name_label());
        }

        let mut ordering = match self.sort_column {
            SortColumn::Name => cmp_insensitive(a_entry.name_label(), b_entry.name_label()),
            SortColumn::Extension => {
                cmp_insensitive(a_entry.extension_label(), b_entry.extension_label())
            }
            SortColumn::Size => size_key(a_entry).cmp(&size_key(b_entry)),
            SortColumn::Date => modified_key(a_entry).cmp(&modified_key(b_entry)),
            SortColumn::Attributes => {
                cmp_insensitive(a_entry.attributes_label(), b_entry.attributes_label())
            }
        };

        if ordering == Ordering::Equal {
            ordering = cmp_insensitive(a_entry.name_label(), b_entry.name_label());
        }

        match self.sort_order {
            SortOrder::Ascending => ordering,
            SortOrder::Descending => ordering.reverse(),
        }
    }

    pub fn status_message(&self, strings: &Strings) -> String {
        match &self.status {
            PanelStatus::Ready => {
                let total = self.filtered_indexes().len();
                let total_str = total.to_string();
                if let Some(entry) = self.selected_entry() {
                    strings
                        .status_selected
                        .replace("{total}", &total_str)
                        .replace("{name}", entry.name.as_str())
                } else {
                    strings
                        .status_items
                        .replace("{total}", &total_str)
                }
            }
            PanelStatus::Loading => strings.status_loading.to_string(),
            PanelStatus::Busy(text) => text.clone(),
            PanelStatus::Error(text) => {
                format!("{}: {text}", strings.status_error_prefix)
            }
        }
    }

    pub fn suggest_new_folder_name(&self, base: &str) -> String {
        if !self.entries.iter().any(|entry| entry.name == base) {
            return base.to_string();
        }
        for idx in 2..1000 {
            let candidate = format!("{base} ({idx})");
            if !self.entries.iter().any(|entry| entry.name == candidate) {
                return candidate;
            }
        }
        format!("{base} ({})", self.entries.len() + 1)
    }

    pub fn clear_rename(&mut self) {
        self.rename_target = None;
        self.rename_input.clear();
    }
}

#[derive(Debug, Clone)]
pub enum PanelStatus {
    Ready,
    Loading,
    Busy(String),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Extension,
    Size,
    Date,
    Attributes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    pub fn toggled(self) -> Self {
        match self {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

impl ThemeMode {
    pub fn toggle(self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UiPreferences {
    text_scale: f32,
    language: Language,
    registration_name: Option<String>,
    registration_input: String,
}

impl UiPreferences {
    pub fn new() -> Self {
        Self {
            text_scale: 1.0,
            language: Language::default(),
            registration_name: None,
            registration_input: String::new(),
        }
    }

    pub fn text_scale(&self) -> f32 {
        self.text_scale
    }

    pub fn scale(&self, value: f32) -> f32 {
        value * self.text_scale
    }

    pub fn increase_text(&mut self) {
        self.text_scale = (self.text_scale + 0.1).min(1.6);
    }

    pub fn decrease_text(&mut self) {
        self.text_scale = (self.text_scale - 0.1).max(0.8);
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    pub fn is_registered(&self) -> bool {
        self.registration_name.is_some()
    }

    pub fn registration_name(&self) -> Option<&str> {
        self.registration_name.as_deref()
    }

    pub fn set_registration_name(&mut self, name: String) {
        self.registration_name = Some(name);
    }

    pub fn clear_registration(&mut self) {
        self.registration_name = None;
    }

    pub fn registration_input(&self) -> &str {
        &self.registration_input
    }

    pub fn set_registration_input(&mut self, value: String) {
        self.registration_input = value;
    }
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DirEntryView {
    pub name: String,
    stem: String,
    extension: String,
    pub is_dir: bool,
    size: Option<u64>,
    modified: Option<SystemTime>,
    attributes: String,
}

impl DirEntryView {
    pub fn from_entry(entry: Entry, root: &Path) -> Self {
        let path_name = Path::new(&entry.name);
        let stem = if entry.is_dir {
            entry.name.clone()
        } else {
            path_name
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&entry.name)
                .to_string()
        };
        let extension = if entry.is_dir {
            String::new()
        } else {
            path_name
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string()
        };
        let metadata = fs::metadata(root.join(&entry.name)).ok();
        let attributes = describe_attributes(metadata.as_ref(), entry.is_dir, &entry.name);
        Self {
            name: entry.name,
            stem,
            extension,
            is_dir: entry.is_dir,
            size: entry.size,
            modified: entry.modified,
            attributes,
        }
    }

    pub fn size_kb_label(&self, strings: &Strings) -> String {
        if self.is_dir {
            strings.dir_label.to_string()
        } else {
            self.size
                .map(|bytes| format!("{:.1} {}", bytes as f64 / 1024.0, strings.size_unit_kb))
                .unwrap_or_else(|| String::from("-"))
        }
    }

    pub fn modified_label(&self) -> String {
        self.modified
            .map(format_timestamp)
            .unwrap_or_else(|| String::from("-"))
    }

    pub fn name_label(&self) -> &str {
        &self.stem
    }

    pub fn display_name(&self) -> String {
        if self.is_dir {
            format!("[{}]", self.name_label())
        } else {
            self.name_label().to_string()
        }
    }

    pub fn extension_label(&self) -> &str {
        &self.extension
    }

    pub fn attributes_label(&self) -> &str {
        &self.attributes
    }
}

#[derive(Debug, Clone)]
pub struct BannerMessage {
    pub severity: BannerSeverity,
    pub text: String,
}

impl BannerMessage {
    pub fn info<T: Into<String>>(text: T) -> Self {
        Self {
            severity: BannerSeverity::Info,
            text: text.into(),
        }
    }

    pub fn error<T: Into<String>>(text: T) -> Self {
        Self {
            severity: BannerSeverity::Error,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BannerSeverity {
    Info,
    Error,
}

#[derive(Debug, Clone)]
pub enum ViewerState {
    Loading { path: PathBuf },
    Ready { path: PathBuf, content: String },
    Error { path: PathBuf, message: String },
}

fn format_timestamp(ts: SystemTime) -> String {
    let datetime: DateTime<Local> = ts.into();
    datetime.format("%y-%m-%d %H:%M").to_string()
}

fn describe_attributes(metadata: Option<&fs::Metadata>, is_dir: bool, name: &str) -> String {
    let mut flags = String::new();
    if is_dir {
        flags.push('D');
    }
    if metadata
        .map(|meta| meta.permissions().readonly())
        .unwrap_or(false)
    {
        flags.push('R');
    }
    if is_hidden(metadata, name) {
        flags.push('H');
    }
    if flags.is_empty() {
        flags.push('-');
    }
    flags
}

#[cfg(windows)]
fn is_hidden(metadata: Option<&fs::Metadata>, _name: &str) -> bool {
    metadata
        .map(|meta| meta.file_attributes() & 0x2 != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_hidden(_metadata: Option<&fs::Metadata>, name: &str) -> bool {
    name.starts_with('.')
}

fn cmp_insensitive(left: &str, right: &str) -> Ordering {
    left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase())
}

fn size_key(entry: &DirEntryView) -> u64 {
    entry.size.unwrap_or(0)
}

fn modified_key(entry: &DirEntryView) -> i128 {
    if let Some(ts) = entry.modified {
        if let Ok(duration) = ts.duration_since(UNIX_EPOCH) {
            return duration.as_nanos() as i128;
        }
    }
    0
}
