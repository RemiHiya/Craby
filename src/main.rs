use buffer::Buffer;
use editor::Editor;

mod editor;
mod buffer;


fn main() -> anyhow::Result<()> {
    let file = std::env::args().nth(1);
    let buffer = Buffer::from_file(file);
    let mut editor = Editor::new(buffer)?;

    editor.run()?;

    Ok(())
}
