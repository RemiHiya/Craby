use std::sync::OnceLock;
use buffer::Buffer;
use editor::Editor;
use logger::Logger;

mod editor;
mod buffer;
mod logger;

static LOGGER: OnceLock<Logger> = OnceLock::new();

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        {
            let log_message = format!($($arg)*);
            $crate::LOGGER.get_or_init(|| $crate::Logger::new("red.log")).log(&log_message);
        }
    };
}

fn main() -> anyhow::Result<()> {
    let file = std::env::args().nth(1);
    let buffer = Buffer::from_file(file);
    let mut editor = Editor::new(buffer)?;

    editor.run()?;

    Ok(())
}
