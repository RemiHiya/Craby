use std::io::{stdout, Stdout, Write};
use crossterm::{terminal, ExecutableCommand, QueueableCommand, cursor, event, style};
use crossterm::event::{read};

enum Action {
    Quit,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,

    AddChar(char),
    NewLine,

    EnterMode(Mode)
}

enum Mode {
    Normal,
    Insert
}

pub struct Editor {
    cx: u16,
    cy: u16,
    mode: Mode
}

impl Editor {
    pub fn new() -> Self {
        Editor {
            cx: 0,
            cy: 0,
            mode: Mode::Normal
        }
    }

    pub fn draw(&self, stdout: &mut Stdout) -> anyhow::Result<()> {
        stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
        stdout.flush()?;
        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut stdout = stdout();
        terminal::enable_raw_mode()?;
        stdout
            .execute(terminal::EnterAlternateScreen)?
            .execute(terminal::Clear(terminal::ClearType::All))?;

        loop {
            self.draw(&mut stdout)?;
            if let Some(action) = self.handle_event(read()?)? {
                match action {
                    Action::Quit => break,
                    Action::MoveUp => self.cy = self.cy.saturating_sub(1),
                    Action::MoveDown => self.cy += 1u16,
                    Action::MoveLeft => self.cx = self.cx.saturating_sub(1),
                    Action::MoveRight => self.cx += 1u16,
                    Action::EnterMode(new) => self.mode = new,
                    Action::AddChar(c) => {
                        stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
                        stdout.queue(style::Print(c))?;
                        self.cx += 1;
                    }
                    Action::NewLine => {
                        self.cx = 0;
                        self.cy += 1;
                    }
                }
            }
        }
        stdout.execute(terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        Ok(())
    }


    fn handle_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>>{
        match self.mode {
            Mode::Normal => self.handle_normal_event(ev),
            Mode::Insert => self.handle_insert_event(ev),
        }
    }

    fn handle_normal_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        match ev {
            event::Event::Key(event::KeyEvent {
                                  code,
                                  kind: event::KeyEventKind::Press,
                                  ..
                              }) => match code {
                event::KeyCode::Char('q') => Ok(Some(Action::Quit)),
                event::KeyCode::Char('h') | event::KeyCode::Left  => Ok(Some(Action::MoveLeft)),
                event::KeyCode::Char('l') | event::KeyCode::Right => Ok(Some(Action::MoveRight)),
                event::KeyCode::Char('k') | event::KeyCode::Up    => Ok(Some(Action::MoveUp)),
                event::KeyCode::Char('j') | event::KeyCode::Down  => Ok(Some(Action::MoveDown)),
                event::KeyCode::Char('i') => Ok(Some(Action::EnterMode(Mode::Insert))),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    fn handle_insert_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
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