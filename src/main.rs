use std::path::PathBuf;

use anyhow::Result;


use iced::executor;
use iced::widget::{button, column, row, scrollable, text, TextInput};
use iced::{Alignment, Application, Command, Element, Length, Settings};

use f::fs as fs_utils;

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
        app.left_entries = fs_utils::list_dir_names(&app.left_path).unwrap_or_default();
        app.right_entries = fs_utils::list_dir_names(&app.right_path).unwrap_or_default();
        (app, Command::none())
    }

    fn title(&self) -> String {
        "f - File Manager".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::RefreshLeft => self.left_entries = fs_utils::list_dir_names(&self.left_path).unwrap_or_default(),
            Message::RefreshRight => self.right_entries = fs_utils::list_dir_names(&self.right_path).unwrap_or_default(),
            Message::ClickLeft(i) => self.left_selected = Some(i),
            Message::ClickRight(i) => self.right_selected = Some(i),
            Message::CopyToRight => {
                if let Some(i) = self.left_selected {
                    if let Some(name) = self.left_entries.get(i) {
                        let src = self.left_path.join(name.trim_end_matches('/'));
                        let _ = fs_utils::copy_to(&src, &self.right_path);
                        self.right_entries = fs_utils::list_dir_names(&self.right_path).unwrap_or_default();
                    }
                }
            }
            Message::CopyToLeft => {
                if let Some(i) = self.right_selected {
                    if let Some(name) = self.right_entries.get(i) {
                        let src = self.right_path.join(name.trim_end_matches('/'));
                        let _ = fs_utils::copy_to(&src, &self.left_path);
                        self.left_entries = fs_utils::list_dir_names(&self.left_path).unwrap_or_default();
                    }
                }
            }
            Message::DeleteLeft => {
                if let Some(i) = self.left_selected {
                    if let Some(name) = self.left_entries.get(i) {
                        let target = self.left_path.join(name.trim_end_matches('/'));
                        let _ = fs_utils::delete(&target);
                        self.left_entries = fs_utils::list_dir_names(&self.left_path).unwrap_or_default();
                        self.left_selected = None;
                    }
                }
            }
            Message::DeleteRight => {
                if let Some(i) = self.right_selected {
                    if let Some(name) = self.right_entries.get(i) {
                        let target = self.right_path.join(name.trim_end_matches('/'));
                        let _ = fs_utils::delete(&target);
                        self.right_entries = fs_utils::list_dir_names(&self.right_path).unwrap_or_default();
                        self.right_selected = None;
                    }
                }
            }
            Message::SearchLeftChanged(s) => {
                self.left_search = s;
                if self.left_search.is_empty() {
                    self.left_entries = fs_utils::list_dir_names(&self.left_path).unwrap_or_default();
                } else {
                    self.left_entries = fs_utils::list_dir_names(&self.left_path).unwrap_or_default().into_iter().filter(|name| name.contains(&self.left_search)).collect::<Vec<String>>();
                }
            }
            Message::SearchRightChanged(s) => {
                self.right_search = s;
                if self.right_search.is_empty() {
                    self.right_entries = fs_utils::list_dir_names(&self.right_path).unwrap_or_default();
                } else {
                    self.right_entries = fs_utils::list_dir_names(&self.right_path).unwrap_or_default().into_iter().filter(|name| name.contains(&self.right_search)).collect::<Vec<String>>();
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
