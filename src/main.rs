use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result as AnyResult;
use f::{Entry, fs as fs_utils};
use humantime::format_rfc3339;
use iced::widget::{
    self, Space, button, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length, Size, Task};

type FsResult<T> = std::result::Result<T, String>;

fn main() -> AnyResult<()> {
    iced::application(
        FileCommander::boot,
        FileCommander::update,
        FileCommander::view,
    )
    .title("f  Dual Pane File Manager")
    .window_size(Size::new(1280.0, 720.0))
    .centered()
    .run()?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

impl Side {
    fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

#[derive(Debug)]
struct FileCommander {
    left: PanelState,
    right: PanelState,
    banner: Option<BannerMessage>,
}

#[derive(Debug, Clone)]
struct PanelState {
    path: PathBuf,
    path_input: String,
    search: String,
    entries: Vec<DirEntryView>,
    selected: Option<usize>,
    status: PanelStatus,
}

impl PanelState {
    fn new(path: PathBuf) -> Self {
        let path_input = path.display().to_string();
        Self {
            path,
            path_input,
            search: String::new(),
            entries: Vec::new(),
            selected: None,
            status: PanelStatus::Ready,
        }
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = path.clone();
        self.path_input = path.display().to_string();
        self.search.clear();
        self.selected = None;
        self.status = PanelStatus::Loading;
    }

    fn can_go_up(&self) -> bool {
        self.path.parent().is_some()
    }

    fn clear_selection(&mut self) {
        self.selected = None;
    }

    fn selected_entry(&self) -> Option<&DirEntryView> {
        self.selected.and_then(|idx| self.entries.get(idx))
    }

    fn filtered_indexes(&self) -> Vec<usize> {
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

    fn status_message(&self) -> String {
        match &self.status {
            PanelStatus::Ready => format!("{} items", self.filtered_indexes().len()),
            PanelStatus::Loading => String::from("Loading..."),
            PanelStatus::Busy(text) => text.clone(),
            PanelStatus::Error(text) => format!("Error: {text}"),
        }
    }
}

#[derive(Debug, Clone)]
enum PanelStatus {
    Ready,
    Loading,
    Busy(String),
    Error(String),
}

#[derive(Debug, Clone)]
struct DirEntryView {
    name: String,
    is_dir: bool,
    size: Option<u64>,
    modified: Option<SystemTime>,
}

impl DirEntryView {
    fn size_label(&self) -> String {
        if self.is_dir {
            String::from("-")
        } else {
            self.size
                .map(human_size)
                .unwrap_or_else(|| String::from("-"))
        }
    }

    fn modified_label(&self) -> String {
        self.modified
            .map(|ts| format_rfc3339(ts).to_string())
            .unwrap_or_else(|| String::from("-"))
    }
}

impl From<Entry> for DirEntryView {
    fn from(value: Entry) -> Self {
        Self {
            name: value.name,
            is_dir: value.is_dir,
            size: value.size,
            modified: value.modified,
        }
    }
}

#[derive(Debug, Clone)]
struct BannerMessage {
    severity: BannerSeverity,
    text: String,
}

impl BannerMessage {
    fn info<T: Into<String>>(text: T) -> Self {
        Self {
            severity: BannerSeverity::Info,
            text: text.into(),
        }
    }

    fn error<T: Into<String>>(text: T) -> Self {
        Self {
            severity: BannerSeverity::Error,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BannerSeverity {
    Info,
    Error,
}

#[derive(Debug, Clone)]
enum Message {
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
    Refresh(Side),
    NavigateUp(Side),
    Copy(Side),
    Delete(Side),
    OperationFinished {
        kind: OperationKind,
        result: FsResult<()>,
    },
    SwapPanels,
}

#[derive(Debug, Clone)]
enum OperationKind {
    Copy { from: Side, to: Side },
    Delete { side: Side },
}

impl FileCommander {
    fn boot() -> (Self, Task<Message>) {
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
            },
            task,
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
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
                panel.entries = entries.into_iter().map(DirEntryView::from).collect();
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
                Task::none()
            }
            Message::PathChanged(side, value) => {
                self.panel_mut(side).path_input = value;
                Task::none()
            }
            Message::PathSubmitted(side) => self.apply_path(side),
            Message::SelectEntry(side, index) => {
                if index < self.panel(side).entries.len() {
                    let already_selected = self.panel(side).selected == Some(index);
                    self.panel_mut(side).selected = Some(index);
                    if already_selected {
                        return self.enter_directory(side, index);
                    }
                }
                Task::none()
            }
            Message::Refresh(side) => self.reload_panel(side),
            Message::NavigateUp(side) => {
                if let Some(parent) = self.panel(side).path.parent().map(|p| p.to_path_buf()) {
                    self.panel_mut(side).set_path(parent.clone());
                    return self.reload_panel(side);
                }
                Task::none()
            }
            Message::Copy(side) => self.copy_from(side),
            Message::Delete(side) => self.delete_from(side),
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
            },
            Message::SwapPanels => {
                std::mem::swap(&mut self.left, &mut self.right);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let panels = row![
            panel_view(&self.left, Side::Left),
            Space::new().width(Length::Fixed(12.0)),
            panel_view(&self.right, Side::Right),
        ]
        .spacing(12)
        .height(Length::Fill);

        let actions = row![
            action_button(
                "Copy ->",
                self.panel(Side::Left)
                    .selected
                    .map(|_| Message::Copy(Side::Left)),
            ),
            action_button(
                "<- Copy",
                self.panel(Side::Right)
                    .selected
                    .map(|_| Message::Copy(Side::Right)),
            ),
            action_button(
                "Delete Left",
                self.panel(Side::Left)
                    .selected
                    .map(|_| Message::Delete(Side::Left)),
            ),
            action_button(
                "Delete Right",
                self.panel(Side::Right)
                    .selected
                    .map(|_| Message::Delete(Side::Right)),
            ),
            button("Swap").on_press(Message::SwapPanels),
        ]
        .spacing(10);

        let banner = banner_view(self.banner.as_ref());

        column![
            panels,
            Space::new().height(Length::Fixed(8.0)),
            actions,
            Space::new().height(Length::Fixed(8.0)),
            banner,
        ]
        .padding(16)
        .spacing(10)
        .into()
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
}

fn panel_view(panel: &PanelState, side: Side) -> Element<'_, Message> {
    let mut up_btn = button("Up");
    if panel.can_go_up() {
        up_btn = up_btn.on_press(Message::NavigateUp(side));
    }

    let header = row![
        text(panel.path.display().to_string())
            .width(Length::Fill)
            .size(16),
        up_btn,
        button("Refresh").on_press(Message::Refresh(side)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let path_input = text_input("Path", &panel.path_input)
        .on_input(move |value| Message::PathChanged(side, value))
        .on_submit(Message::PathSubmitted(side));

    let search_input = text_input("Filter", &panel.search)
        .on_input(move |value| Message::SearchChanged(side, value));

    let list_header = row![
        text("#").width(Length::Fixed(24.0)).size(13),
        text("Name").width(Length::FillPortion(5)).size(13),
        text("Size").width(Length::FillPortion(2)).size(13),
        text("Modified").width(Length::FillPortion(3)).size(13),
    ];

    let rows = panel.filtered_indexes().into_iter().fold(
        column![container(list_header).padding(6)],
        |col, idx| {
            let entry = &panel.entries[idx];
            let indicator = if panel.selected == Some(idx) { ">" } else { "" };
            let display_name = if entry.is_dir {
                format!("[DIR] {}", entry.name)
            } else {
                entry.name.clone()
            };
            let row_content = row![
                text(indicator).width(Length::Fixed(24.0)),
                text(display_name).width(Length::FillPortion(5)),
                text(entry.size_label()).width(Length::FillPortion(2)),
                text(entry.modified_label()).width(Length::FillPortion(3)),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let clickable = mouse_area(container(row_content).padding(6))
                .on_press(Message::SelectEntry(side, idx));

            col.push(clickable)
        },
    );

    let list = scrollable(rows).height(Length::Fill);
    let status = text(panel.status_message());

    column![header, path_input, search_input, list, status]
        .spacing(8)
        .width(Length::FillPortion(1))
        .into()
}

fn action_button(label: &str, message: Option<Message>) -> widget::Button<'_, Message> {
    let btn = button(label);
    match message {
        Some(msg) => btn.on_press(msg),
        None => btn,
    }
}

fn banner_view(message: Option<&BannerMessage>) -> Element<'_, Message> {
    let text_widget = match message {
        Some(msg) => {
            let prefix = match msg.severity {
                BannerSeverity::Info => "[info]",
                BannerSeverity::Error => "[error]",
            };
            text(format!("{prefix} {}", msg.text))
        }
        None => text("Ready"),
    };
    container(text_widget).width(Length::Fill).padding(8).into()
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

fn human_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn load_panel(side: Side, path: PathBuf) -> Task<Message> {
    Task::perform(list_entries(path.clone()), move |result| match result {
        Ok(entries) => Message::PanelLoaded {
            side,
            path,
            entries,
        },
        Err(error) => Message::PanelLoadFailed { side, path, error },
    })
}

async fn list_entries(path: PathBuf) -> FsResult<Vec<Entry>> {
    fs_utils::list_dir(&path).map_err(|err| err.to_string())
}

async fn copy_entry(src: PathBuf, dest_dir: PathBuf) -> FsResult<()> {
    fs_utils::copy_to(&src, &dest_dir).map_err(|err| err.to_string())
}

async fn delete_entry(target: PathBuf) -> FsResult<()> {
    fs_utils::delete(&target).map_err(|err| err.to_string())
}
