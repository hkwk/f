use iced::border::Border;
use iced::widget::container::Style;
use iced::widget::text::Style as TextStyle;
use iced::widget::{
    self, Space, button, column, container, mouse_area, pick_list, row, scrollable, text,
    text_input,
};
use iced::{Alignment, Color, Element, Length};

use crate::i18n::{Language, Strings};
use crate::messages::Message;
use crate::model::{
    BannerMessage, BannerSeverity, PanelState, Side, SortColumn, SortOrder, ThemeMode,
    UiPreferences, ViewerState,
};

const TEXT_SIZE_BASE: f32 = 13.0;
const TEXT_SIZE_SMALL: f32 = 11.0;
const TEXT_SIZE_TITLE: f32 = 14.0;
const PANEL_SPACING: f32 = 6.0;
const PANEL_SPACING_TIGHT: f32 = 4.0;
const PANEL_PADDING: u16 = 8;
const ROW_PADDING: u16 = 4;
const TOOLBAR_WIDTH: f32 = 88.0;
const VIEWER_HEIGHT: f32 = 180.0;
const GAP_MEDIUM: f32 = 6.0;
const GAP_SMALL: f32 = 4.0;

pub fn build_layout<'a>(
    left: &'a PanelState,
    right: &'a PanelState,
    active: Side,
    banner: Option<&'a BannerMessage>,
    viewer: Option<&'a ViewerState>,
    theme_mode: ThemeMode,
    show_config: bool,
    show_about: bool,
    prefs: &'a UiPreferences,
    strings: &'static Strings,
) -> Element<'a, Message> {
    let palette = palette(theme_mode);
    let active_panel = match active {
        Side::Left => left,
        Side::Right => right,
    };

    let mut layout = column![top_toolbar(
        active,
        active_panel,
        palette,
        prefs,
        theme_mode,
        show_config,
        show_about,
        strings,
    )]
    .spacing(PANEL_SPACING)
    .width(Length::Fill);

    if show_config {
        layout = layout.push(config_panel(prefs, palette, strings));
    }

    if show_about {
        layout = layout.push(about_panel(palette, prefs, strings));
    }

    let panels = row![
        panel_view(left, Side::Left, active == Side::Left, palette, prefs, strings),
        center_toolbar(active_panel, active, palette, prefs, strings),
        panel_view(right, Side::Right, active == Side::Right, palette, prefs, strings),
    ]
    .spacing(PANEL_SPACING)
    .height(Length::Fill);

    let actions = row![
        action_button(
            strings.action_copy_right,
            left.selected.map(|_| Message::Copy(Side::Left)),
            palette,
            prefs,
        ),
        action_button(
            strings.action_copy_left,
            right.selected.map(|_| Message::Copy(Side::Right)),
            palette,
            prefs,
        ),
        action_button(
            strings.action_delete_left,
            left.selected.map(|_| Message::Delete(Side::Left)),
            palette,
            prefs,
        ),
        action_button(
            strings.action_delete_right,
            right.selected.map(|_| Message::Delete(Side::Right)),
            palette,
            prefs,
        ),
        action_button(strings.action_swap, Some(Message::SwapPanels), palette, prefs),
    ]
    .spacing(PANEL_SPACING);

    let function_keys = function_key_row(
        active_panel.selected.map(|_| Message::Copy(active)),
        active_panel.selected.map(|_| Message::Move(active)),
        Some(Message::CreateFolder(active)),
        active_panel.selected.map(|_| Message::Delete(active)),
        Some(Message::Refresh(active)),
        palette,
        prefs,
        strings,
    );

    let mut content = layout
        .push(Space::new().height(Length::Fixed(GAP_SMALL)))
        .push(panels)
        .push(Space::new().height(Length::Fixed(GAP_MEDIUM)))
        .push(actions)
        .push(Space::new().height(Length::Fixed(GAP_MEDIUM)))
        .push(function_keys);

    if let Some(viewer_state) = viewer {
        content = content
            .push(Space::new().height(Length::Fixed(GAP_SMALL)))
            .push(viewer_view(viewer_state, palette, prefs, strings));
    }

    content = content
        .push(Space::new().height(Length::Fixed(GAP_SMALL)))
        .push(banner_view(banner, palette, prefs, strings));

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| Style {
            background: Some(palette.background.into()),
            ..Style::default()
        })
        .into()
}

fn top_toolbar(
    active: Side,
    panel: &PanelState,
    palette: UiPalette,
    prefs: &UiPreferences,
    theme_mode: ThemeMode,
    show_config: bool,
    show_about: bool,
    strings: &'static Strings,
) -> Element<'static, Message> {
    let can_hash = panel
        .selected_entry()
        .map(|entry| !entry.is_dir)
        .unwrap_or(false);

    let refresh = top_button(strings.toolbar_refresh, Some(Message::RefreshAll), palette, prefs);
    let mut hash_btn = top_button(strings.toolbar_hashes, None, palette, prefs);
    if can_hash {
        hash_btn = hash_btn.on_press(Message::ComputeHashes(active));
    }
    let theme_label = match theme_mode {
        ThemeMode::Light => strings.toolbar_theme_dark,
        ThemeMode::Dark => strings.toolbar_theme_light,
    };
    let theme_btn = top_button(theme_label, Some(Message::ToggleTheme), palette, prefs);

    let config_label = if show_config {
        format!("{}*", strings.toolbar_config)
    } else {
        strings.toolbar_config.to_string()
    };
    let about_label = if show_about {
        format!("{}*", strings.toolbar_about)
    } else {
        strings.toolbar_about.to_string()
    };

    let config_btn = top_button(config_label, Some(Message::ToggleConfigPanel), palette, prefs);
    let about_btn = top_button(about_label, Some(Message::ToggleAboutPanel), palette, prefs);
    let badge_element: Option<Element<'static, Message>> = if prefs.is_registered() {
        None
    } else {
        let badge_content = column![
            label(
                strings.badge_open_source,
                scaled(TEXT_SIZE_BASE, prefs),
                palette.text,
            ),
            label(
                strings.registration_cta,
                scaled(TEXT_SIZE_SMALL, prefs),
                palette.muted_text,
            ),
        ]
        .spacing(2);
        Some(
            container(mouse_area(badge_content).on_press(Message::ToggleConfigPanel))
                .padding([4, 8])
                .style(move |_| Style {
                    background: Some(palette.surface.into()),
                    border: Border {
                        width: 1.0,
                        radius: 6.0.into(),
                        color: palette.border_inactive,
                    },
                    ..Style::default()
                })
                .into(),
        )
    };

    let mut row = row![
        refresh,
        hash_btn,
        theme_btn,
        config_btn,
        Space::new().width(Length::Fill),
    ]
    .spacing(PANEL_SPACING)
    .align_y(Alignment::Center);

    if let Some(badge) = badge_element {
        row = row
            .push(badge)
            .push(Space::new().width(Length::Fixed(8.0)));
    }
    row = row.push(about_btn);

    container(row)
        .padding(ROW_PADDING)
        .style(move |_| Style {
            background: Some(palette.surface.into()),
            border: Border {
                width: 0.0,
                radius: 6.0.into(),
                color: palette.surface,
            },
            ..Style::default()
        })
        .into()
}

fn config_panel(
    prefs: &UiPreferences,
    palette: UiPalette,
    strings: &'static Strings,
) -> Element<'static, Message> {
    let label_text = label(
        strings.config_font_size,
        scaled(TEXT_SIZE_BASE, prefs),
        palette.text,
    );
    let percent = format!("{:.0}%", prefs.text_scale() * 100.0);
    let size_value = label(percent, scaled(TEXT_SIZE_BASE, prefs), palette.accent);
    let language_label = label(
        strings.config_language,
        scaled(TEXT_SIZE_BASE, prefs),
        palette.text,
    );
    let language_picker = pick_list(
        Language::all(),
        Some(prefs.language()),
        Message::LanguageSelected,
    )
    .placeholder(strings.config_language_placeholder);

    let base_row = row![
        label_text,
        Space::new().width(Length::Fixed(8.0)),
        top_button("-", Some(Message::DecreaseFontSize), palette, prefs),
        size_value,
        top_button("+", Some(Message::IncreaseFontSize), palette, prefs),
        Space::new().width(Length::Fixed(16.0)),
        language_label,
        language_picker,
        Space::new().width(Length::Fill),
        top_button(strings.button_close, Some(Message::ToggleConfigPanel), palette, prefs),
    ]
    .spacing(PANEL_SPACING)
    .align_y(Alignment::Center);

    let status_text = if let Some(name) = prefs.registration_name() {
        strings
            .registration_status_registered
            .replace("{name}", name)
    } else {
        strings.registration_status_open.to_string()
    };
    let status_color = if prefs.is_registered() {
        palette.accent
    } else {
        palette.muted_text
    };
    let status_row = row![
        label(
            format!("{}: {}", strings.registration_heading, status_text),
            scaled(TEXT_SIZE_BASE, prefs),
            status_color,
        )
        .width(Length::Fill),
    ];

    let registration_input = text_input(
        strings.registration_placeholder,
        prefs.registration_input(),
    )
    .on_input(Message::RegistrationCodeChanged)
    .on_submit(Message::ApplyRegistration)
    .width(Length::FillPortion(3));

    let apply_btn = top_button(strings.registration_apply, Some(Message::ApplyRegistration), palette, prefs);
    let mut clear_btn = top_button(strings.registration_clear, None, palette, prefs);
    if prefs.is_registered() {
        clear_btn = clear_btn.on_press(Message::ClearRegistration);
    }

    let registration_row = row![
        registration_input,
        apply_btn,
        clear_btn,
    ]
    .spacing(PANEL_SPACING)
    .align_y(Alignment::Center);

    let content = column![
        base_row,
        Space::new().height(Length::Fixed(GAP_SMALL)),
        status_row,
        Space::new().height(Length::Fixed(GAP_SMALL)),
        registration_row,
    ]
    .spacing(PANEL_SPACING);

    container(content)
        .padding(ROW_PADDING)
        .style(move |_| Style {
            background: Some(palette.surface.into()),
            border: Border {
                width: 1.0,
                radius: 6.0.into(),
                color: palette.border_inactive,
            },
            ..Style::default()
        })
        .into()
}

fn about_panel(
    palette: UiPalette,
    prefs: &UiPreferences,
    strings: &'static Strings,
) -> Element<'static, Message> {
    let edition_value = if prefs.is_registered() {
        strings.badge_registered
    } else {
        strings.badge_open_source
    };
    let edition_line = label(
        format!("{}: {}", strings.registration_status_label, edition_value),
        scaled(TEXT_SIZE_BASE, prefs),
        palette.text,
    );
    let status_line = if let Some(name) = prefs.registration_name() {
        strings
            .registration_status_registered
            .replace("{name}", name)
    } else {
        strings.registration_status_open.to_string()
    };
    let lines = column![
        edition_line,
        label(status_line, scaled(TEXT_SIZE_SMALL, prefs), palette.muted_text),
        label(
            strings.about_project,
            scaled(TEXT_SIZE_BASE, prefs),
            palette.text,
        ),
        label(strings.about_author, scaled(TEXT_SIZE_BASE, prefs), palette.text),
        label(
            strings.about_license,
            scaled(TEXT_SIZE_BASE, prefs),
            palette.text,
        ),
    ]
    .spacing(PANEL_SPACING_TIGHT);

    let content = row![
        lines,
        Space::new().width(Length::Fill),
        top_button(strings.button_close, Some(Message::ToggleAboutPanel), palette, prefs),
    ]
    .align_y(Alignment::Start)
    .spacing(PANEL_SPACING);

    container(content)
        .padding(ROW_PADDING)
        .style(move |_| Style {
            background: Some(palette.surface.into()),
            border: Border {
                width: 1.0,
                radius: 6.0.into(),
                color: palette.border_inactive,
            },
            ..Style::default()
        })
        .into()
}

fn function_key_row(
    copy: Option<Message>,
    move_op: Option<Message>,
    mkdir: Option<Message>,
    delete: Option<Message>,
    refresh: Option<Message>,
    palette: UiPalette,
    prefs: &UiPreferences,
    strings: &'static Strings,
) -> Element<'static, Message> {
    row![
        key_button(strings.function_f5_copy, copy, palette, prefs),
        key_button(strings.function_f6_move, move_op, palette, prefs),
        key_button(strings.function_f7_mkdir, mkdir, palette, prefs),
        key_button(strings.function_f8_delete, delete, palette, prefs),
        key_button(strings.function_f2_refresh, refresh, palette, prefs),
    ]
    .spacing(PANEL_SPACING)
    .width(Length::Fill)
    .into()
}

fn key_button(
    caption: &'static str,
    message: Option<Message>,
    palette: UiPalette,
    prefs: &UiPreferences,
) -> Element<'static, Message> {
    let content = label(caption, scaled(TEXT_SIZE_BASE, prefs), palette.text)
        .width(Length::Fill);
    let mut btn = button(content).padding(6).width(Length::FillPortion(1));
    if let Some(msg) = message {
        btn = btn.on_press(msg);
    }
    btn.into()
}

fn panel_view<'a>(
    panel: &'a PanelState,
    side: Side,
    is_active: bool,
    palette: UiPalette,
    prefs: &'a UiPreferences,
    strings: &'static Strings,
) -> Element<'a, Message> {
    let mut up_btn = button(strings.panel_up).padding(4);
    if panel.can_go_up() {
        up_btn = up_btn.on_press(Message::NavigateUp(side));
    }

    let header = row![
        label(
            panel.path.display().to_string(),
            scaled(TEXT_SIZE_TITLE, prefs),
            palette.text,
        )
        .width(Length::Fill),
        up_btn,
        button(strings.panel_refresh)
            .padding(4)
            .on_press(Message::Refresh(side)),
    ]
    .spacing(PANEL_SPACING)
    .align_y(Alignment::Center);

    let path_input = text_input(strings.input_path, &panel.path_input)
        .on_input(move |value| Message::PathChanged(side, value))
        .on_submit(Message::PathSubmitted(side));

    let search_input = text_input(strings.input_filter, &panel.search)
        .on_input(move |value| Message::SearchChanged(side, value));

    let new_folder_row = {
        let folder_input = text_input(strings.input_new_folder, &panel.new_folder)
            .on_input(move |value| Message::NewFolderNameChanged(side, value));
        let mut create_btn = button(strings.button_create).padding(4);
        if !panel.new_folder.trim().is_empty() {
            create_btn = create_btn.on_press(Message::CreateFolder(side));
        }
        row![folder_input, create_btn]
            .spacing(PANEL_SPACING_TIGHT)
            .align_y(Alignment::Center)
    };

    let rename_row = panel.rename_target.map(|_| {
        let rename_input = text_input(strings.input_rename_to, &panel.rename_input)
            .on_input(move |value| Message::RenameInputChanged(side, value))
            .on_submit(Message::SubmitRename(side));
        let mut confirm_btn = button(strings.button_rename).padding(4);
        if !panel.rename_input.trim().is_empty() {
            confirm_btn = confirm_btn.on_press(Message::SubmitRename(side));
        }
        let cancel_btn = button(strings.button_cancel)
            .padding(4)
            .on_press(Message::CancelRename(side));
        row![rename_input, confirm_btn, cancel_btn]
            .spacing(PANEL_SPACING_TIGHT)
            .align_y(Alignment::Center)
    });

    let list_header = container(header_row(panel, side, palette, prefs, strings)).padding(ROW_PADDING);

    let mut rows = column![];

    if panel.can_go_up() {
        rows = rows.push(parent_navigation_row(panel.parent_selected, side, palette, prefs));
    }

    let rows = panel.visible_indexes().into_iter().fold(rows, |col, idx| {
        let entry = &panel.entries[idx];
        let prefix = if panel.selected == Some(idx) { "> " } else { "  " };
        let name_label = entry.display_name();
        let row_content = row![
            label(
                format!("| {prefix}{name_label}"),
                scaled(TEXT_SIZE_BASE, prefs),
                palette.text,
            )
            .width(Length::FillPortion(5)),
            label(
                format!("| {}", entry.extension_label()),
                scaled(TEXT_SIZE_BASE, prefs),
                palette.text,
            )
            .width(Length::FillPortion(2)),
            label(
                format!("| {}", entry.size_kb_label(strings)),
                scaled(TEXT_SIZE_BASE, prefs),
                palette.text,
            )
            .width(Length::FillPortion(2)),
            label(
                format!("| {}", entry.modified_label()),
                scaled(TEXT_SIZE_BASE, prefs),
                palette.text,
            )
            .width(Length::FillPortion(3)),
            label(
                format!("| {} |", entry.attributes_label()),
                scaled(TEXT_SIZE_BASE, prefs),
                palette.text,
            )
            .width(Length::FillPortion(2)),
        ]
        .spacing(PANEL_SPACING_TIGHT)
        .align_y(Alignment::Center);

        let clickable = mouse_area(container(row_content).padding(ROW_PADDING))
            .on_press(Message::SelectEntry(side, idx));

        col.push(clickable)
    });

    let list = scrollable(rows).height(Length::Fill);
    let status = label(
        panel.status_message(strings),
        scaled(TEXT_SIZE_SMALL, prefs),
        palette.muted_text,
    );
    let mut content = column![header, path_input, search_input, new_folder_row]
        .spacing(PANEL_SPACING)
        .width(Length::Fill);

    if let Some(row) = rename_row {
        content = content.push(row);
    }

    content = content.push(list_header).push(list).push(status);

    container(content)
        .padding(PANEL_PADDING)
        .width(Length::FillPortion(1))
        .style(move |_| panel_container_style(is_active, palette))
        .into()
}

fn panel_container_style(is_active: bool, palette: UiPalette) -> Style {
    let background = if is_active {
        palette.panel_active
    } else {
        palette.panel_inactive
    };
    let border = if is_active {
        palette.border_active
    } else {
        palette.border_inactive
    };
    Style {
        background: Some(background.into()),
        border: Border {
            width: if is_active { 2.0 } else { 1.0 },
            radius: 8.0.into(),
            color: border,
        },
        ..Style::default()
    }
}

fn center_toolbar<'a>(
    panel: &'a PanelState,
    side: Side,
    palette: UiPalette,
    prefs: &'a UiPreferences,
    strings: &'static Strings,
) -> Element<'a, Message> {
    let selected = panel.selected_entry();
    let has_selection = selected.is_some();
    let file_selected = selected.map(|entry| !entry.is_dir).unwrap_or(false);

    let buttons = column![
        toolbar_button(
            "[V]",
            strings.center_view,
            if file_selected {
                Some(Message::ViewFile(side))
            } else {
                None
            },
            palette,
            prefs,
        ),
        toolbar_button(
            "[E]",
            strings.center_edit,
            if file_selected {
                Some(Message::EditFile(side))
            } else {
                None
            },
            palette,
            prefs,
        ),
        toolbar_button(
            "[C]",
            strings.center_copy,
            has_selection.then(|| Message::Copy(side)),
            palette,
            prefs,
        ),
        toolbar_button(
            "[R]",
            strings.center_rename,
            has_selection.then(|| Message::StartRename(side)),
            palette,
            prefs,
        ),
        toolbar_button(
            "[Z]",
            strings.center_zip,
            has_selection.then(|| Message::Compress(side)),
            palette,
            prefs,
        ),
        toolbar_button(
            "[+]",
            strings.center_folder,
            Some(Message::QuickCreateFolder(side)),
            palette,
            prefs,
        ),
    ]
    .spacing(PANEL_SPACING)
    .width(Length::Shrink);

    container(buttons)
        .width(Length::Shrink)
        .padding(ROW_PADDING)
        .style(move |_| Style {
            background: Some(palette.surface.into()),
            border: Border {
                width: 1.0,
                radius: 6.0.into(),
                color: palette.border_inactive,
            },
            ..Style::default()
        })
        .into()
}

fn toolbar_button(
    icon: &'static str,
    label_text: &'static str,
    message: Option<Message>,
    palette: UiPalette,
    prefs: &UiPreferences,
) -> Element<'static, Message> {
    let content = column![
        label(icon, scaled(TEXT_SIZE_TITLE, prefs), palette.text),
        label(label_text, scaled(TEXT_SIZE_SMALL, prefs), palette.muted_text),
    ]
    .spacing(2);
    let mut btn = button(content)
        .padding(4)
        .width(Length::Fixed(TOOLBAR_WIDTH));
    if let Some(msg) = message {
        btn = btn.on_press(msg);
    }
    btn.into()
}

fn viewer_view<'a>(
    state: &'a ViewerState,
    palette: UiPalette,
    prefs: &'a UiPreferences,
    strings: &'static Strings,
) -> Element<'a, Message> {
    let format_path_label = |template: &'static str, path: &std::path::PathBuf| {
        let rendered = path.display().to_string();
        template.replace("{path}", &rendered)
    };
    let title = match state {
        ViewerState::Loading { path } => format_path_label(strings.viewer_title, path),
        ViewerState::Ready { path, .. } => format_path_label(strings.viewer_title, path),
        ViewerState::Error { path, .. } => format_path_label(strings.viewer_error_title, path),
    };

    let body: Element<'_, Message> = match state {
        ViewerState::Loading { .. } => {
            label(strings.viewer_loading, scaled(TEXT_SIZE_BASE, prefs), palette.text).into()
        }
        ViewerState::Ready { content, .. } => scrollable(
            label(content.clone(), scaled(TEXT_SIZE_BASE, prefs), palette.text),
        )
        .into(),
        ViewerState::Error { message, .. } => {
            label(message.clone(), scaled(TEXT_SIZE_BASE, prefs), palette.text).into()
        }
    };

    let header = row![
        label(title, scaled(TEXT_SIZE_TITLE, prefs), palette.text)
            .width(Length::Fill),
        button(strings.button_close)
            .padding(4)
            .on_press(Message::CloseViewer),
    ]
    .spacing(PANEL_SPACING)
    .align_y(Alignment::Center);

    let body_container = container(body)
        .height(Length::Fixed(VIEWER_HEIGHT))
        .width(Length::Fill)
        .style(move |_| Style {
            background: Some(palette.panel_inactive.into()),
            border: Border {
                width: 1.0,
                radius: 6.0.into(),
                color: palette.border_inactive,
            },
            ..Style::default()
        });

    container(column![header, body_container].spacing(PANEL_SPACING))
        .width(Length::Fill)
        .padding(PANEL_PADDING)
        .style(move |_| Style {
            background: Some(palette.surface.into()),
            border: Border {
                width: 1.0,
                radius: 8.0.into(),
                color: palette.border_inactive,
            },
            ..Style::default()
        })
        .into()
}

fn action_button(
    caption: &'static str,
    message: Option<Message>,
    palette: UiPalette,
    prefs: &UiPreferences,
) -> widget::Button<'static, Message> {
    let text = label(caption, scaled(TEXT_SIZE_BASE, prefs), palette.text);
    let mut btn = button(text).padding(6);
    if let Some(msg) = message {
        btn = btn.on_press(msg);
    }
    btn
}

fn banner_view<'a>(
    message: Option<&'a BannerMessage>,
    palette: UiPalette,
    prefs: &'a UiPreferences,
    strings: &'static Strings,
) -> Element<'a, Message> {
    let text_widget = match message {
        Some(msg) => {
            let prefix = match msg.severity {
                BannerSeverity::Info => strings.banner_info_prefix,
                BannerSeverity::Error => strings.banner_error_prefix,
            };
            label(
                format!("{prefix} {}", msg.text),
                scaled(TEXT_SIZE_SMALL, prefs),
                palette.text,
            )
        }
        None => label(strings.banner_ready, scaled(TEXT_SIZE_SMALL, prefs), palette.muted_text),
    };
    container(text_widget)
        .width(Length::Fill)
        .padding(ROW_PADDING)
        .style(move |_| Style {
            background: Some(palette.surface.into()),
            border: Border {
                width: 0.0,
                radius: 6.0.into(),
                color: palette.surface,
            },
            ..Style::default()
        })
        .into()
}

fn header_row<'a>(
    panel: &'a PanelState,
    side: Side,
    palette: UiPalette,
    prefs: &'a UiPreferences,
    strings: &'static Strings,
) -> Element<'a, Message> {
    row![
        sortable_header(
            format!("| {}", strings.header_name),
            Length::FillPortion(5),
            SortColumn::Name,
            panel,
            side,
            palette,
            prefs,
        ),
        sortable_header(
            format!("| {}", strings.header_extension),
            Length::FillPortion(2),
            SortColumn::Extension,
            panel,
            side,
            palette,
            prefs,
        ),
        sortable_header(
            format!("| {}", strings.header_size),
            Length::FillPortion(2),
            SortColumn::Size,
            panel,
            side,
            palette,
            prefs,
        ),
        sortable_header(
            format!("| {}", strings.header_date),
            Length::FillPortion(3),
            SortColumn::Date,
            panel,
            side,
            palette,
            prefs,
        ),
        sortable_header(
            format!("| {} |", strings.header_attrs),
            Length::FillPortion(2),
            SortColumn::Attributes,
            panel,
            side,
            palette,
            prefs,
        ),
    ]
    .spacing(PANEL_SPACING_TIGHT)
    .align_y(Alignment::Center)
    .into()
}

fn sortable_header(
    label_text: impl Into<String>,
    width: Length,
    column: SortColumn,
    panel: &PanelState,
    side: Side,
    palette: UiPalette,
    prefs: &UiPreferences,
) -> Element<'static, Message> {
    let mut text_label = label_text.into();
    if panel.sort_column == column {
        let arrow = match panel.sort_order {
            SortOrder::Ascending => " ↑",
            SortOrder::Descending => " ↓",
        };
        text_label.push_str(arrow);
    }

    let content = label(text_label, scaled(TEXT_SIZE_SMALL, prefs), palette.muted_text)
        .width(Length::Fill);

    button(content)
        .padding(2)
        .width(width)
        .on_press(Message::ToggleSort(side, column))
        .into()
}

fn parent_navigation_row(
    parent_selected: bool,
    side: Side,
    palette: UiPalette,
    prefs: &UiPreferences,
) -> Element<'static, Message> {
    let prefix = if parent_selected { "> " } else { "  " };
    let row_content = row![
        label(
            format!("| {prefix}[..]"),
            scaled(TEXT_SIZE_BASE, prefs),
            palette.text,
        )
        .width(Length::FillPortion(5)),
        label("|", scaled(TEXT_SIZE_BASE, prefs), palette.text)
            .width(Length::FillPortion(2)),
        label("|", scaled(TEXT_SIZE_BASE, prefs), palette.text)
            .width(Length::FillPortion(2)),
        label("|", scaled(TEXT_SIZE_BASE, prefs), palette.text)
            .width(Length::FillPortion(3)),
        label("| |", scaled(TEXT_SIZE_BASE, prefs), palette.text)
            .width(Length::FillPortion(2)),
    ]
    .spacing(PANEL_SPACING_TIGHT)
    .align_y(Alignment::Center);

    mouse_area(container(row_content).padding(ROW_PADDING))
        .on_press(Message::SelectParent(side))
        .into()
}

fn top_button(
    caption: impl Into<String>,
    message: Option<Message>,
    palette: UiPalette,
    prefs: &UiPreferences,
) -> widget::Button<'static, Message> {
    let text = label(caption.into(), scaled(TEXT_SIZE_BASE, prefs), palette.text);
    let mut btn = button(text).padding(6);
    if let Some(msg) = message {
        btn = btn.on_press(msg);
    }
    btn
}

fn scaled(size: f32, prefs: &UiPreferences) -> f32 {
    prefs.scale(size)
}

fn label<'a>(content: impl Into<String>, size: f32, color: Color) -> iced::widget::Text<'a> {
    text(content.into())
        .size(size)
        .style(move |_| TextStyle {
            color: Some(color),
            ..Default::default()
        })
}

#[derive(Clone, Copy)]
struct UiPalette {
    background: Color,
    surface: Color,
    panel_active: Color,
    panel_inactive: Color,
    border_active: Color,
    border_inactive: Color,
    text: Color,
    muted_text: Color,
    accent: Color,
}

fn palette(mode: ThemeMode) -> UiPalette {
    match mode {
        ThemeMode::Light => UiPalette {
            background: Color::from_rgb8(247, 248, 250),
            surface: Color::from_rgb8(232, 235, 240),
            panel_active: Color::from_rgb8(255, 255, 255),
            panel_inactive: Color::from_rgb8(244, 246, 249),
            border_active: Color::from_rgb8(79, 70, 229),
            border_inactive: Color::from_rgb8(209, 213, 219),
            text: Color::from_rgb8(31, 41, 55),
            muted_text: Color::from_rgb8(107, 114, 128),
            accent: Color::from_rgb8(99, 102, 241),
        },
        ThemeMode::Dark => UiPalette {
            background: Color::from_rgb8(17, 24, 39),
            surface: Color::from_rgb8(31, 41, 55),
            panel_active: Color::from_rgb8(45, 55, 72),
            panel_inactive: Color::from_rgb8(34, 44, 60),
            border_active: Color::from_rgb8(96, 165, 250),
            border_inactive: Color::from_rgb8(55, 65, 81),
            text: Color::from_rgb8(243, 244, 246),
            muted_text: Color::from_rgb8(156, 163, 175),
            accent: Color::from_rgb8(129, 140, 248),
        },
    }
}
