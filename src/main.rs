mod app;
mod i18n;
mod messages;
mod model;
mod ops;
mod types;
mod ui;

use anyhow::Result as AnyResult;
use iced::Size;

use crate::app::FileCommander;

fn main() -> AnyResult<()> {
    iced::application(
        FileCommander::boot,
        FileCommander::update,
        FileCommander::view,
    )
    .subscription(FileCommander::subscription)
    .title("f  Dual Pane File Manager")
    .window_size(Size::new(1280.0, 720.0))
    .antialiasing(true)
    .centered()
    .run()?;
    Ok(())
}
