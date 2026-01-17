use std::path::{Path, PathBuf};
use std::process::Command;

use iced::event::{self, Status};
use iced::keyboard::{self, key::Named, Key, Modifiers};
use iced::window;
use iced::{Element, Event, Subscription, Task};

use crate::i18n;
use crate::messages::{Message, OperationKind};
use crate::model::{
    BannerMessage, DirEntryView, PanelState, PanelStatus, Side, ThemeMode, UiPreferences,
    ViewerState,
};
use crate::ops::{
    compress_path, compute_hashes, copy_entry, create_directory, delete_entry, load_panel,
    move_entry, read_text_file, rename_path,
};
use crate::ui;
use f::registration;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug)]
pub struct FileCommander {
    left: PanelState,
    right: PanelState,
    banner: Option<BannerMessage>,
    active: Side,
    viewer: Option<ViewerState>,
    theme: ThemeMode,
    show_config: bool,
    show_about: bool,
    ui_prefs: UiPreferences,
}

impl FileCommander {
    pub fn boot() -> (Self, Task<Message>) {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut left = PanelState::new(cwd.clone());
        let mut right = PanelState::new(cwd.clone());
        left.status = PanelStatus::Loading;
        right.status = PanelStatus::Loading;
        let task = Task::batch(vec![
            load_panel(Side::Left, left.path.clone()),
            load_panel(Side::Right, right.path.clone()),
        ]);
        (
            Self {
                left,
                right,
                banner: None,
                active: Side::Left,
                viewer: None,
                theme: ThemeMode::Light,
                show_config: false,
                show_about: false,
                ui_prefs: UiPreferences::new(),
            },
            task,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PanelLoaded {
                side,
                path,
                entries,
            } => {
                if self.panel(side).path != path {
                    return Task::none();
                }
                let panel = self.panel_mut(side);
                let root = panel.path.clone();
                panel.entries = entries
                    .into_iter()
                    .map(|entry| DirEntryView::from_entry(entry, &root))
                    .collect();
                panel.status = PanelStatus::Ready;
                panel.selected = None;
                Task::none()
            }
            Message::PanelLoadFailed { side, path, error } => {
                if self.panel(side).path != path {
                    return Task::none();
                }
                self.panel_mut(side).status = PanelStatus::Error(error.clone());
                self.banner = Some(BannerMessage::error(format!(
                    "{side:?} load failed: {error}"
                )));
                Task::none()
            }
            Message::SearchChanged(side, query) => {
                let panel = self.panel_mut(side);
                panel.search = query;
                panel.clear_selection();
                self.active = side;
                Task::none()
            }
            Message::PathChanged(side, value) => {
                self.panel_mut(side).path_input = value;
                self.active = side;
                Task::none()
            }
            Message::PathSubmitted(side) => {
                self.active = side;
                self.apply_path(side)
            }
            Message::SelectEntry(side, index) => {
                if index < self.panel(side).entries.len() {
                    let already_selected = self.panel(side).selected == Some(index);
                    let panel = self.panel_mut(side);
                    panel.selected = Some(index);
                    panel.parent_selected = false;
                    panel.clear_rename();
                    self.active = side;
                    if already_selected {
                        return self.enter_directory(side, index);
                    }
                }
                Task::none()
            }
            Message::SelectParent(side) => {
                if !self.panel(side).can_go_up() {
                    return Task::none();
                }
                let already_selected = self.panel(side).parent_selected;
                {
                    let panel = self.panel_mut(side);
                    panel.selected = None;
                    panel.parent_selected = true;
                    panel.clear_rename();
                }
                self.active = side;
                if already_selected {
                    return self.go_up(side);
                }
                Task::none()
            }
            Message::ToggleSort(side, column) => {
                self.active = side;
                self.panel_mut(side).toggle_sort(column);
                Task::none()
            }
            Message::Refresh(side) => self.reload_panel(side),
            Message::RefreshAll => self.refresh_all(),
            Message::NavigateUp(side) => self.go_up(side),
            Message::Copy(side) => self.copy_from(side),
            Message::Move(side) => self.move_from(side),
            Message::Delete(side) => self.delete_from(side),
            Message::NewFolderNameChanged(side, value) => {
                self.panel_mut(side).new_folder = value;
                self.active = side;
                Task::none()
            }
            Message::CreateFolder(side) => {
                self.active = side;
                self.create_folder(side)
            }
            Message::QuickCreateFolder(side) => {
                self.active = side;
                self.quick_create_folder(side)
            }
            Message::StartRename(side) => {
                self.active = side;
                self.begin_rename(side);
                Task::none()
            }
            Message::RenameInputChanged(side, value) => {
                let panel = self.panel_mut(side);
                if panel.rename_target.is_some() {
                    panel.rename_input = value;
                }
                Task::none()
            }
            Message::SubmitRename(side) => {
                self.active = side;
                self.rename_entry(side)
            }
            Message::CancelRename(side) => {
                self.panel_mut(side).clear_rename();
                Task::none()
            }
            Message::OperationFinished { kind, result } => match (kind, result) {
                (OperationKind::Copy { from, to }, Ok(())) => {
                    self.panel_mut(from).status = PanelStatus::Ready;
                    self.banner = Some(BannerMessage::info("Copy completed"));
                    Task::batch(vec![self.reload_panel(from), self.reload_panel(to)])
                }
                (OperationKind::Copy { from, .. }, Err(err)) => {
                    self.panel_mut(from).status = PanelStatus::Error(err.clone());
                    self.banner = Some(BannerMessage::error(err));
                    Task::none()
                }
                (OperationKind::Move { from, to }, Ok(())) => {
                    self.panel_mut(from).status = PanelStatus::Ready;
                    self.banner = Some(BannerMessage::info("Move completed"));
                    Task::batch(vec![self.reload_panel(from), self.reload_panel(to)])
                }
                (OperationKind::Move { from, .. }, Err(err)) => {
                    self.panel_mut(from).status = PanelStatus::Error(err.clone());
                    self.banner = Some(BannerMessage::error(err));
                    Task::none()
                }
                (OperationKind::Delete { side }, Ok(())) => {
                    self.panel_mut(side).status = PanelStatus::Ready;
                    self.banner = Some(BannerMessage::info("Delete completed"));
                    self.reload_panel(side)
                }
                (OperationKind::Delete { side }, Err(err)) => {
                    self.panel_mut(side).status = PanelStatus::Error(err.clone());
                    self.banner = Some(BannerMessage::error(err));
                    Task::none()
                }
                (OperationKind::CreateFolder { side }, Ok(())) => {
                    self.panel_mut(side).status = PanelStatus::Ready;
                    self.banner = Some(BannerMessage::info("Folder created"));
                    self.reload_panel(side)
                }
                (OperationKind::CreateFolder { side }, Err(err)) => {
                    self.panel_mut(side).status = PanelStatus::Error(err.clone());
                    self.banner = Some(BannerMessage::error(err));
                    Task::none()
                }
                (OperationKind::Rename { side }, Ok(())) => {
                    {
                        let panel = self.panel_mut(side);
                        panel.status = PanelStatus::Ready;
                        panel.clear_rename();
                    }
                    self.banner = Some(BannerMessage::info("Rename completed"));
                    self.reload_panel(side)
                }
                (OperationKind::Rename { side }, Err(err)) => {
                    self.panel_mut(side).status = PanelStatus::Error(err.clone());
                    self.banner = Some(BannerMessage::error(err));
                    Task::none()
                }
                (OperationKind::Compress { side, dest }, Ok(())) => {
                    self.panel_mut(side).status = PanelStatus::Ready;
                    self.banner = Some(BannerMessage::info(format!(
                        "Created archive {}",
                        dest.display()
                    )));
                    self.reload_panel(side)
                }
                (OperationKind::Compress { side, .. }, Err(err)) => {
                    self.panel_mut(side).status = PanelStatus::Error(err.clone());
                    self.banner = Some(BannerMessage::error(err));
                    Task::none()
                }
            },
            Message::ViewFile(side) => {
                self.active = side;
                self.open_viewer(side)
            }
            Message::ViewerLoaded { path, result } => {
                let matches_path = matches!(
                    &self.viewer,
                    Some(ViewerState::Loading { path: current }) if *current == path
                );
                if !matches_path {
                    return Task::none();
                }
                self.viewer = Some(match result {
                    Ok(content) => ViewerState::Ready { path, content },
                    Err(message) => ViewerState::Error { path, message },
                });
                Task::none()
            }
            Message::CloseViewer => {
                self.viewer = None;
                Task::none()
            }
            Message::EditFile(side) => {
                self.active = side;
                self.launch_editor(side)
            }
            Message::Compress(side) => {
                self.active = side;
                self.compress_from(side)
            }
            Message::FocusPanel(side) => {
                self.active = side;
                Task::none()
            }
            Message::SwapPanels => {
                std::mem::swap(&mut self.left, &mut self.right);
                self.active = self.active.other();
                Task::none()
            }
            Message::ComputeHashes(side) => {
                self.active = side;
                self.compute_hashes(side)
            }
            Message::HashesComputed { side, path, result } => {
                self.panel_mut(side).status = PanelStatus::Ready;
                match result {
                    Ok(hashes) => {
                        let report = hashes.to_report(&path);
                        self.banner = Some(BannerMessage::info("Hash calculation complete"));
                        self.viewer = Some(ViewerState::Ready { path, content: report });
                    }
                    Err(err) => {
                        self.banner = Some(BannerMessage::error(err));
                    }
                }
                Task::none()
            }
            Message::ToggleTheme => {
                self.theme = self.theme.toggle();
                Task::none()
            }
            Message::ToggleConfigPanel => {
                self.show_config = !self.show_config;
                if self.show_config {
                    self.show_about = false;
                }
                Task::none()
            }
            Message::ToggleAboutPanel => {
                self.show_about = !self.show_about;
                if self.show_about {
                    self.show_config = false;
                }
                Task::none()
            }
            Message::IncreaseFontSize => {
                self.ui_prefs.increase_text();
                Task::none()
            }
            Message::DecreaseFontSize => {
                self.ui_prefs.decrease_text();
                Task::none()
            }
            Message::MoveSelection { side, delta } => {
                self.active = side;
                self.panel_mut(side).move_selection(delta);
                Task::none()
            }
            Message::LanguageSelected(language) => {
                self.ui_prefs.set_language(language);
                Task::none()
            }
            Message::RegistrationCodeChanged(value) => {
                self.ui_prefs.set_registration_input(value);
                Task::none()
            }
            Message::ApplyRegistration => {
                let code = self.ui_prefs.registration_input().trim().to_string();
                if code.is_empty() {
                    self.banner = Some(BannerMessage::error("Enter a registration code"));
                    return Task::none();
                }
                match registration::verify_code(&code) {
                    Ok(info) => {
                        self.ui_prefs.set_registration_name(info.name.clone());
                        self.ui_prefs.set_registration_input(String::new());
                        self.banner = Some(BannerMessage::info(format!(
                            "Registered to {}",
                            info.name
                        )));
                    }
                    Err(err) => {
                        self.banner = Some(BannerMessage::error(format!(
                            "Registration failed: {err}"
                        )));
                    }
                }
                Task::none()
            }
            Message::ClearRegistration => {
                if self.ui_prefs.is_registered() {
                    self.ui_prefs.clear_registration();
                    self.banner = Some(BannerMessage::info("Registration cleared"));
                }
                Task::none()
            }
            Message::KeyboardShortcut { key, modifiers } => {
                if let Some(mapped) = shortcut_message(key, modifiers, self.active) {
                    return self.update(mapped);
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let language = self.ui_prefs.language();
        let strings = i18n::strings(language);
        ui::build_layout(
            &self.left,
            &self.right,
            self.active,
            self.banner.as_ref(),
            self.viewer.as_ref(),
            self.theme,
            self.show_config,
            self.show_about,
            &self.ui_prefs,
            strings,
        )
    }

    pub fn window_title(&self) -> String {
        let strings = i18n::strings(self.ui_prefs.language());
        let edition = if self.ui_prefs.is_registered() {
            strings.badge_registered
        } else {
            strings.badge_open_source
        };
        format!("{} {}", strings.title_base, edition)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        event::listen_with(keyboard_shortcut_listener)
    }

    fn panel(&self, side: Side) -> &PanelState {
        match side {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }

    fn panel_mut(&mut self, side: Side) -> &mut PanelState {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    fn reload_panel(&mut self, side: Side) -> Task<Message> {
        let path = self.panel(side).path.clone();
        self.panel_mut(side).status = PanelStatus::Loading;
        load_panel(side, path)
    }

    fn apply_path(&mut self, side: Side) -> Task<Message> {
        let input = self.panel(side).path_input.clone();
        let candidate = resolve_path(&self.panel(side).path, &input);
        match candidate {
            Some(path) if path.is_dir() => {
                self.panel_mut(side).set_path(path.clone());
                self.reload_panel(side)
            }
            _ => {
                self.panel_mut(side).status = PanelStatus::Error(String::from("Invalid path"));
                Task::none()
            }
        }
    }

    fn enter_directory(&mut self, side: Side, index: usize) -> Task<Message> {
        if let Some(entry) = self.panel(side).entries.get(index) {
            if entry.is_dir {
                let mut next = self.panel(side).path.clone();
                next.push(&entry.name);
                self.panel_mut(side).set_path(next);
                return self.reload_panel(side);
            }
        }
        Task::none()
    }

    fn copy_from(&mut self, from: Side) -> Task<Message> {
        let Some(entry) = self.panel(from).selected_entry() else {
            return Task::none();
        };
        let src = self.panel(from).path.join(&entry.name);
        let dest_dir = self.panel(from.other()).path.clone();
        self.panel_mut(from).status = PanelStatus::Busy(String::from("Copying..."));
        Task::perform(copy_entry(src, dest_dir), move |result| {
            Message::OperationFinished {
                kind: OperationKind::Copy {
                    from,
                    to: from.other(),
                },
                result,
            }
        })
    }

    fn move_from(&mut self, from: Side) -> Task<Message> {
        let Some(entry) = self.panel(from).selected_entry() else {
            return Task::none();
        };
        let src = self.panel(from).path.join(&entry.name);
        let dest_dir = self.panel(from.other()).path.clone();
        self.panel_mut(from).status = PanelStatus::Busy(String::from("Moving..."));
        Task::perform(move_entry(src, dest_dir), move |result| {
            Message::OperationFinished {
                kind: OperationKind::Move {
                    from,
                    to: from.other(),
                },
                result,
            }
        })
    }

    fn delete_from(&mut self, side: Side) -> Task<Message> {
        let Some(entry) = self.panel(side).selected_entry() else {
            return Task::none();
        };
        let target = self.panel(side).path.join(&entry.name);
        self.panel_mut(side).status = PanelStatus::Busy(String::from("Deleting..."));
        Task::perform(delete_entry(target), move |result| {
            Message::OperationFinished {
                kind: OperationKind::Delete { side },
                result,
            }
        })
    }

    fn create_folder(&mut self, side: Side) -> Task<Message> {
        let folder_name = self.panel(side).new_folder.trim().to_string();
        if folder_name.is_empty() {
            self.banner = Some(BannerMessage::error("Enter a folder name"));
            return Task::none();
        }
        let target = self.panel(side).path.join(folder_name);
        let panel = self.panel_mut(side);
        panel.new_folder.clear();
        panel.status = PanelStatus::Busy(String::from("Creating folder..."));
        Task::perform(create_directory(target), move |result| {
            Message::OperationFinished {
                kind: OperationKind::CreateFolder { side },
                result,
            }
        })
    }

    fn quick_create_folder(&mut self, side: Side) -> Task<Message> {
        let strings = i18n::strings(self.ui_prefs.language());
        let name = self
            .panel(side)
            .suggest_new_folder_name(strings.new_folder_suggestion);
        self.panel_mut(side).new_folder = name;
        self.create_folder(side)
    }

    fn begin_rename(&mut self, side: Side) {
        let Some(idx) = self.panel(side).selected else {
            self.banner = Some(BannerMessage::error("Select an item to rename"));
            return;
        };
        let entry_name = match self.panel(side).entries.get(idx) {
            Some(entry) => entry.name.clone(),
            None => return,
        };
        let panel = self.panel_mut(side);
        panel.rename_target = Some(idx);
        panel.rename_input = entry_name;
    }

    fn rename_entry(&mut self, side: Side) -> Task<Message> {
        let Some(target_idx) = self.panel(side).rename_target else {
            self.banner = Some(BannerMessage::error("No item selected for rename"));
            return Task::none();
        };
        let new_name = self.panel(side).rename_input.trim().to_string();
        if new_name.is_empty() {
            self.banner = Some(BannerMessage::error("Enter a new name"));
            return Task::none();
        }
        let (entry_name, base_path) = {
            let panel = self.panel(side);
            let Some(entry) = panel.entries.get(target_idx) else {
                return Task::none();
            };
            (entry.name.clone(), panel.path.clone())
        };
        let src = base_path.join(&entry_name);
        self.panel_mut(side).status = PanelStatus::Busy(String::from("Renaming..."));
        Task::perform(rename_path(src, new_name), move |result| {
            Message::OperationFinished {
                kind: OperationKind::Rename { side },
                result,
            }
        })
    }

    fn open_viewer(&mut self, side: Side) -> Task<Message> {
        let (is_dir, entry_name, base_path) = {
            let panel = self.panel(side);
            let Some(entry) = panel.selected_entry() else {
                self.banner = Some(BannerMessage::error("Select a file to view"));
                return Task::none();
            };
            (entry.is_dir, entry.name.clone(), panel.path.clone())
        };
        if is_dir {
            self.banner = Some(BannerMessage::error("Cannot view a directory"));
            return Task::none();
        }
        let path = base_path.join(&entry_name);
        self.viewer = Some(ViewerState::Loading { path: path.clone() });
        Task::perform(read_text_file(path.clone()), move |result| {
            Message::ViewerLoaded { path, result }
        })
    }

    fn launch_editor(&mut self, side: Side) -> Task<Message> {
        let (is_dir, entry_name, base_path) = {
            let panel = self.panel(side);
            let Some(entry) = panel.selected_entry() else {
                self.banner = Some(BannerMessage::error("Select a file to edit"));
                return Task::none();
            };
            (entry.is_dir, entry.name.clone(), panel.path.clone())
        };
        if is_dir {
            self.banner = Some(BannerMessage::error("Cannot edit a directory"));
            return Task::none();
        }
        let path = base_path.join(&entry_name);
        match open_in_default_app(&path) {
            Ok(()) => {
                self.banner = Some(BannerMessage::info(format!("Opened {}", entry_name)));
            }
            Err(err) => {
                self.banner = Some(BannerMessage::error(err));
            }
        }
        Task::none()
    }

    fn compress_from(&mut self, side: Side) -> Task<Message> {
        let (entry_name, base_path) = {
            let panel = self.panel(side);
            let Some(entry) = panel.selected_entry() else {
                self.banner = Some(BannerMessage::error("Select an item to compress"));
                return Task::none();
            };
            (entry.name.clone(), panel.path.clone())
        };
        let src = base_path.join(&entry_name);
        let dest = next_archive_path(&src);
        self.panel_mut(side).status = PanelStatus::Busy(String::from("Compressing..."));
        Task::perform(compress_path(src, dest.clone()), move |result| {
            Message::OperationFinished {
                kind: OperationKind::Compress { side, dest },
                result,
            }
        })
    }

    fn refresh_all(&mut self) -> Task<Message> {
        Task::batch(vec![self.reload_panel(Side::Left), self.reload_panel(Side::Right)])
    }

    fn compute_hashes(&mut self, side: Side) -> Task<Message> {
        let (is_dir, entry_name, base_path) = {
            let panel = self.panel(side);
            let Some(entry) = panel.selected_entry() else {
                self.banner = Some(BannerMessage::error("Select a file to hash"));
                return Task::none();
            };
            (entry.is_dir, entry.name.clone(), panel.path.clone())
        };
        if is_dir {
            self.banner = Some(BannerMessage::error("Cannot hash a directory"));
            return Task::none();
        }
        let path = base_path.join(&entry_name);
        self.panel_mut(side).status = PanelStatus::Busy(String::from("Hashing..."));
        Task::perform(compute_hashes(path.clone()), move |result| {
            Message::HashesComputed { side, path, result }
        })
    }

    fn go_up(&mut self, side: Side) -> Task<Message> {
        if let Some(parent) = self.panel(side).path.parent().map(|p| p.to_path_buf()) {
            self.panel_mut(side).set_path(parent);
            self.panel_mut(side).parent_selected = false;
            self.active = side;
            return self.reload_panel(side);
        }
        Task::none()
    }
}

fn keyboard_shortcut_listener(
    event: Event,
    status: Status,
    _window: window::Id,
) -> Option<Message> {
    if status == Status::Captured {
        return None;
    }

    match event {
        Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            Some(Message::KeyboardShortcut { key, modifiers })
        }
        _ => None,
    }
}

fn shortcut_message(key: Key, modifiers: Modifiers, active: Side) -> Option<Message> {
    let command_pressed = modifiers.control() || modifiers.command();

    match key {
        Key::Named(named) => match named {
            Named::Escape => Some(Message::CloseViewer),
            Named::Delete => Some(Message::Delete(active)),
            Named::Tab => Some(Message::FocusPanel(active.other())),
            Named::ArrowUp => Some(Message::MoveSelection {
                side: active,
                delta: -1,
            }),
            Named::ArrowDown => Some(Message::MoveSelection {
                side: active,
                delta: 1,
            }),
            Named::F2 => Some(Message::Refresh(active)),
            Named::F3 => Some(Message::ViewFile(active)),
            Named::F4 => Some(Message::EditFile(active)),
            Named::F5 => Some(Message::Copy(active)),
            Named::F6 => Some(Message::Move(active)),
            Named::F7 => Some(Message::CreateFolder(active)),
            Named::F8 => Some(Message::Delete(active)),
            _ => None,
        },
        Key::Character(value) => {
            let lower = value.to_lowercase();
            if command_pressed {
                match lower.as_str() {
                    "h" => return Some(Message::ComputeHashes(active)),
                    "r" if modifiers.shift() => return Some(Message::RefreshAll),
                    "t" => return Some(Message::ToggleTheme),
                    "," => return Some(Message::ToggleConfigPanel),
                    "." => return Some(Message::ToggleAboutPanel),
                    "u" => return Some(Message::SwapPanels),
                    _ => {}
                }
            }

            match lower.as_str() {
                "v" => Some(Message::ViewFile(active)),
                "e" => Some(Message::EditFile(active)),
                "c" => Some(Message::Copy(active)),
                "r" => Some(Message::StartRename(active)),
                "z" => Some(Message::Compress(active)),
                "+" => Some(Message::QuickCreateFolder(active)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn resolve_path(current: &Path, input: &str) -> Option<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        Some(candidate)
    } else {
        Some(current.join(candidate))
    }
}

fn next_archive_path(src: &Path) -> PathBuf {
    let parent = src.parent().unwrap_or_else(|| Path::new("."));
    let base_name = if src.is_dir() {
        src.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("archive")
    } else {
        src.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("archive")
    };
    let mut candidate = parent.join(format!("{base_name}.zip"));
    let mut counter = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{base_name} ({counter}).zip"));
        counter += 1;
    }
    candidate
}

fn open_in_default_app(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let target = path.to_string_lossy().to_string();
        Command::new("cmd")
            .args(["/C", "start", "", &target])
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|err| err.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|err| err.to_string())?;
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|err| err.to_string())?;
    }

    Ok(())
}
