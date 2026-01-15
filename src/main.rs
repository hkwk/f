use std::fs;
use std::path::{PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

use iced::executor;
use iced::widget::{button, column, row, scrollable, text, TextInput};
use iced::{Alignment, Application, Command, Element, Length, Settings};

fn list_dir(path: &PathBuf) -> Vec<String> {
    let mut entries = Vec::new();
    if let Ok(read) = fs::read_dir(path) {
        for entry in read.flatten() {
            if let Ok(file_type) = entry.file_type() {
                let name = entry.file_name().to_string_lossy().to_string();
                if file_type.is_dir() {
                    entries.push(format!("{}/", name));
                } else {
                    entries.push(name);
                }
            }
        }
    }
    entries.sort();
    entries
}

#[derive(Debug, Clone)]
enum Message {
    RefreshLeft,
    RefreshRight,
    ClickLeft(usize),
    ClickRight(usize),
    CopyToRight,
    CopyToLeft,
    DeleteLeft,
    DeleteRight,
    SearchLeftChanged(String),
    SearchRightChanged(String),
}

struct AppState {
    left_path: PathBuf,
    right_path: PathBuf,

    left_entries: Vec<String>,
    right_entries: Vec<String>,

    left_selected: Option<usize>,
    right_selected: Option<usize>,

    left_search: String,
    right_search: String,
}

impl Default for AppState {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            left_path: cwd.clone(),
            right_path: cwd,
            left_entries: Vec::new(),
            right_entries: Vec::new(),
            left_selected: None,
            right_selected: None,
            left_search: String::new(),
            right_search: String::new(),
        }
    }
}

impl Application for AppState {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = ();
    type Theme = iced::Theme;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let mut app = AppState::default();
        app.left_entries = list_dir(&app.left_path);
        app.right_entries = list_dir(&app.right_path);
        (app, Command::none())
    }

    fn title(&self) -> String {
        "f - File Manager".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::RefreshLeft => self.left_entries = list_dir(&self.left_path),
            Message::RefreshRight => self.right_entries = list_dir(&self.right_path),
            Message::ClickLeft(i) => self.left_selected = Some(i),
            Message::ClickRight(i) => self.right_selected = Some(i),
            Message::CopyToRight => {
                if let Some(i) = self.left_selected {
                    if let Some(name) = self.left_entries.get(i) {
                        let src = self.left_path.join(name.trim_end_matches('/'));
                        if src.is_dir() {
                            let _ = fs_extra::dir::copy(&src, &self.right_path, &fs_extra::dir::CopyOptions::new());
                        } else if let Some(file_name) = src.file_name() {
                            let dest = self.right_path.join(file_name);
                            let _ = fs::copy(&src, &dest);
                        }
                        self.right_entries = list_dir(&self.right_path);
                    }
                }
            }
            Message::CopyToLeft => {
                if let Some(i) = self.right_selected {
                    if let Some(name) = self.right_entries.get(i) {
                        let src = self.right_path.join(name.trim_end_matches('/'));
                        if src.is_dir() {
                            let _ = fs_extra::dir::copy(&src, &self.left_path, &fs_extra::dir::CopyOptions::new());
                        } else if let Some(file_name) = src.file_name() {
                            let dest = self.left_path.join(file_name);
                            let _ = fs::copy(&src, &dest);
                        }
                        self.left_entries = list_dir(&self.left_path);
                    }
                }
            }
            Message::DeleteLeft => {
                if let Some(i) = self.left_selected {
                    if let Some(name) = self.left_entries.get(i) {
                        let target = self.left_path.join(name.trim_end_matches('/'));
                        let _ = if target.is_dir() { fs::remove_dir_all(&target) } else { fs::remove_file(&target) };
                        self.left_entries = list_dir(&self.left_path);
                        self.left_selected = None;
                    }
                }
            }
            Message::DeleteRight => {
                if let Some(i) = self.right_selected {
                    if let Some(name) = self.right_entries.get(i) {
                        let target = self.right_path.join(name.trim_end_matches('/'));
                        let _ = if target.is_dir() { fs::remove_dir_all(&target) } else { fs::remove_file(&target) };
                        self.right_entries = list_dir(&self.right_path);
                        self.right_selected = None;
                    }
                }
            }
            Message::SearchLeftChanged(s) => {
                self.left_search = s;
                if self.left_search.is_empty() {
                    self.left_entries = list_dir(&self.left_path);
                } else {
                    self.left_entries = list_dir(&self.left_path).into_iter().filter(|name| name.contains(&self.left_search)).collect();
                }
            }
            Message::SearchRightChanged(s) => {
                self.right_search = s;
                if self.right_search.is_empty() {
                    self.right_entries = list_dir(&self.right_path);
                } else {
                    self.right_entries = list_dir(&self.right_path).into_iter().filter(|name| name.contains(&self.right_search)).collect();
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let left_list = self.left_entries.iter().enumerate().fold(column![].spacing(5), |col, (i, name)| {
            let btn = button(text(name)).on_press(Message::ClickLeft(i)).width(Length::Fill);
            col.push(btn)
        });

        let right_list = self.right_entries.iter().enumerate().fold(column![].spacing(5), |col, (i, name)| {
            let btn = button(text(name)).on_press(Message::ClickRight(i)).width(Length::Fill);
            col.push(btn)
        });

        let left_panel = column![
            text(format!("Left: {}", self.left_path.display())),
            TextInput::new(&self.left_search, &self.left_search).on_input(Message::SearchLeftChanged),
            scrollable(left_list).height(Length::FillPortion(1)),
            row![
                button("Copy →").on_press(Message::CopyToRight),
                button("Del").on_press(Message::DeleteLeft),
                button("Refresh").on_press(Message::RefreshLeft),
            ]
            .spacing(10)
        ]
        .spacing(10)
        .width(Length::FillPortion(1));

        let right_panel = column![
            text(format!("Right: {}", self.right_path.display())),
            TextInput::new(&self.right_search, &self.right_search).on_input(Message::SearchRightChanged),
            scrollable(right_list).height(Length::FillPortion(1)),
            row![
                button("← Copy").on_press(Message::CopyToLeft),
                button("Del").on_press(Message::DeleteRight),
                button("Refresh").on_press(Message::RefreshRight),
            ]
            .spacing(10)
        ]
        .spacing(10)
        .width(Length::FillPortion(1));

        let content = row![left_panel, right_panel].spacing(20).align_items(Alignment::Start);

        content.into()
    }
}

fn main() -> Result<()> {
    let settings = Settings::default();
    AppState::run(settings)?;
    Ok(())
}
