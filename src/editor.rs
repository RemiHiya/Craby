use std::io::{stdout, Stdout, Write};
use crossterm::{terminal, ExecutableCommand, QueueableCommand, cursor, event, style};
use crossterm::event::{read};
use crossterm::style::{Color, Stylize};
use crate::buffer::Buffer;

enum Action {
    Quit,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    PageDown,

    AddChar(char),
    NewLine,

    EnterMode(Mode),
    PageUp,
    MoveToLineEnd,
    MoveToLineStart,
}

#[derive(Debug)]
enum Mode {
    Normal,
    Insert
}

pub struct Editor {
    buffer: Buffer,
    stdout: Stdout,
    size: (u16, u16),
    vtop: u16,
    vleft: u16,
    cx: u16,
    cy: u16,
    mode: Mode
}

impl Drop for Editor {
    fn drop(&mut self) {
        _ = self.stdout.flush();
        _ = self.stdout.execute(terminal::LeaveAlternateScreen);
        _ = terminal::disable_raw_mode();
    }
}

impl Editor {
    pub fn new(buffer: Buffer) -> anyhow::Result<Self> {
        let mut stdout = stdout();
        terminal::enable_raw_mode()?;
        stdout
            .execute(terminal::EnterAlternateScreen)?
            .execute(terminal::Clear(terminal::ClearType::All))?;
        let size = terminal::size()?;
        Ok(Editor {
            buffer,
            stdout,
            size,
            vtop: 0,
            vleft: 0,
            cx: 0,
            cy: 0,
            mode: Mode::Normal
        })
    }

    fn vwidth(&self) -> u16 {
        self.size.0
    }

    fn vheight(&self) -> u16 {
        self.size.1 - 2
    }

    fn line_length(&self) -> u16 {
        if let Some(line) = self.viewport_line(self.cy) {
            return line.len() as u16;
        }
        0
    }

    fn buffer_line(&self) -> u16 {
        self.vtop + self.cy
    }

    fn viewport_line(&self, n: u16) -> Option<String> {
        let buffer_line = self.vtop + n;
        self.buffer.get(buffer_line as usize)
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        self.draw_viewport()?;
        self.draw_statusline()?;
        self.stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn draw_viewport(&mut self) -> anyhow::Result<()> {
        let vwidth = self.vwidth() as usize;
        for i in 0..self.vheight() {
            let line = self.viewport_line(i).unwrap_or_else(|| String::new());
            self.stdout
                .queue(cursor::MoveTo(0, i))?
                .queue(style::Print(format!("{line:<width$}", width=vwidth)))?;
        }
        Ok(())
    }

    pub fn draw_statusline(&mut self) -> anyhow::Result<()> {
        let mode = format!(" {:?} ", self.mode).to_uppercase();
        let file = format!(" {}", self.buffer.file.as_deref().unwrap_or("untitled"));
        let pos = format!(" {}:{} ", self.cx, self.cy);

        let file_width = self.size.0 - mode.len() as u16 - pos.len() as u16 - 2;

        let normal_bg = Color::Rgb {r:184, g:144, b:243};
        //let insert_bg = 1;
        let classic_bg = Color::Rgb {r:67, g:70, b:89};

        self.stdout.queue(cursor::MoveTo(0, self.size.1 - 2))?;
        self.stdout.queue(style::PrintStyledContent(
            mode.with(Color::Black).bold().on(normal_bg)
        ))?;
        self.stdout.queue(style::PrintStyledContent("".with(normal_bg).on(classic_bg)))?;
        self.stdout.queue(style::PrintStyledContent(
            format!("{:<width$}", file, width = file_width as usize)
                .with(Color::White)
                .bold()
                .on(classic_bg),
        ))?;
        self.stdout.queue(style::PrintStyledContent(
           "".with(normal_bg).on(classic_bg)
        ))?;
        self.stdout.queue(style::PrintStyledContent(
            pos.with(Color::Black).on(normal_bg)
        ))?;
        Ok(())
    }

    fn check_bounds(&mut self) {
        let l = self.line_length();
        if self.cx >= l {
            if l > 0 {
                self.cx = self.line_length() - 1;
            } else {
                self.cx = 0;
            }
        }
        if self.cx >= self.vwidth() {
            self.cx = self.vwidth() - 1;
        }

        let line_on_buffer = self.cy + self.vtop;
        if line_on_buffer as usize >= self.buffer.len() {
            self.cy = self.buffer.len() as u16 - self.vtop - 1;
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.check_bounds();
            self.draw()?;
            if let Some(action) = self.handle_event(read()?)? {
                match action {
                    Action::Quit => break,
                    Action::MoveUp => {
                        if self.cy == 0 {
                            // Scroll up
                            if self.vtop > 0 {
                                self.vtop -= 1;
                            }
                        } else {
                            self.cy = self.cy.saturating_sub(1)
                        }
                    },
                    Action::MoveDown => {
                        self.cy += 1;
                        if self.cy >= self.vheight() {
                            // Scroll down
                            self.vtop += 1;
                            self.cy = self.vheight() - 1;
                        }
                    },
                    Action::MoveLeft => {
                        self.cx = self.cx.saturating_sub(1);
                        if self.cx < self.vleft {
                            self.cx = self.vleft;
                        }
                    },
                    Action::MoveRight => {
                        self.cx += 1;
                    },
                    Action::PageUp => {
                        self.vtop = self.vtop.saturating_sub(self.vheight())
                    }
                    Action::PageDown => {
                        if self.buffer.len() > (self.vtop + self.vheight()) as usize {
                            self.vtop += self.vheight();
                        } else {
                            self.vtop = self.buffer.len() as u16 - 1;
                        }
                    }
                    Action::MoveToLineEnd => {
                        self.cx = self.line_length().saturating_sub(1);
                    }
                    Action::MoveToLineStart => {
                        self.cx = 0;
                    }
                    Action::EnterMode(new) => self.mode = new,
                    Action::AddChar(c) => {
                        self.buffer.insert(self.cx, self.buffer_line(), c);
                        self.cx += 1;
                    }
                    Action::NewLine => {
                        self.cx = 0;
                        self.cy += 1;
                    }
                }
            }
        }
        Ok(())
    }


    fn handle_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>>{
        // if matches!(ev, event::Event::Resize(_, _)) {
        //     self.size = terminal::size()?;
        // }
        match self.mode {
            Mode::Normal => self.handle_normal_event(ev),
            Mode::Insert => self.handle_insert_event(ev),
        }
    }

    fn handle_normal_event(&self, ev: event::Event) -> anyhow::Result<Option<Action>> {
    match ev {
        event::Event::Key(event::KeyEvent {
                              code,
                              kind: event::KeyEventKind::Press,
                              modifiers,
                              ..
                          }) => match code {
            event::KeyCode::Char('q') => Ok(Some(Action::Quit)),
            event::KeyCode::Char('$') => Ok(Some(Action::MoveToLineEnd)),
            event::KeyCode::Char('0') => Ok(Some(Action::MoveToLineStart)),
            event::KeyCode::Char('h') | event::KeyCode::Left  => Ok(Some(Action::MoveLeft)),
            event::KeyCode::Char('l') | event::KeyCode::Right => Ok(Some(Action::MoveRight)),
            event::KeyCode::Char('k') | event::KeyCode::Up    => Ok(Some(Action::MoveUp)),
            event::KeyCode::Char('j') | event::KeyCode::Down  => Ok(Some(Action::MoveDown)),
            event::KeyCode::Char('i')              => Ok(Some(Action::EnterMode(Mode::Insert))),
            event::KeyCode::Char('f') if matches!(modifiers, event::KeyModifiers::CONTROL) => {
                Ok(Some(Action::PageDown))
            },
            event::KeyCode::Char('b') if matches!(modifiers, event::KeyModifiers::CONTROL) => {
                Ok(Some(Action::PageUp))
            },
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}

    fn handle_insert_event(&self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        match ev {
            event::Event::Key(event::KeyEvent {
                                  code,
                                  kind: event::KeyEventKind::Press,
                                  .. }) => match code {
                event::KeyCode::Esc => Ok(Some(Action::EnterMode(Mode::Normal))),
                event::KeyCode::Char(c) =>  Ok(Some(Action::AddChar(c))),
                _ => Ok(None)
            }
            _ => Ok(None)
        }
    }
}