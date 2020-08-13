#![warn(rust_2018_idioms)]
use std::fmt::Write;
use std::io;
use std::sync::{
    mpsc::{self, TryRecvError},
    Arc, Mutex,
};
use std::thread;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::text::Span;
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::Terminal;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn convert_to_binary(text: &[u8]) -> Result<String> {
    let mut out = String::with_capacity(text.len() * 2);
    for b in text {
        write!(&mut out, "{:x}", b)?;
    }
    Ok(out)
}

#[derive(Debug)]
enum Event {
    Quit,
    ScrollUp,
    ScrollDown,
}

struct App {
    buffer: String,
}

impl App {
    fn run(&mut self) -> Result<()> {
        let stdout = io::stdout().into_raw_mode()?;
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        let mut running = true;
        let mut scroll = 0;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let stdin = io::stdin();
            for event in stdin.keys() {
                if let Ok(key) = event {
                    if key == Key::Char('q') {
                        tx.send(Event::Quit).unwrap();
                    } else if key == Key::Up {
                        tx.send(Event::ScrollUp).unwrap();
                    } else if key == Key::Down {
                        tx.send(Event::ScrollDown).unwrap();
                    }
                }
            }
        });

        loop {
            if !running {
                break;
            }

            loop {
                match rx.try_recv() {
                    Ok(Event::Quit) => running = false,
                    Ok(Event::ScrollUp) => scroll = if scroll == 0 { 0 } else { scroll - 1 },
                    Ok(Event::ScrollDown) => scroll += 1,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => panic!("events channel disconnected"),
                }
            }

            terminal.draw(|f| {
                let size = f.size();
                let create_block = |title| {
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default())
                        .title(Span::styled(
                            title,
                            Style::default().add_modifier(Modifier::BOLD),
                        ))
                };

                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(size);
                let paragraph = Paragraph::new(self.buffer.as_str())
                    .style(Style::default())
                    .block(create_block("text"))
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: true })
                    .scroll((scroll, 0));
                f.render_widget(paragraph, chunks[0]);
            })?;
        }
        Ok(())
    }
}

fn main() {
    let text = std::fs::read("src/main.rs").expect("reading text into string");
    let binary = convert_to_binary(&text).expect("converting to binary");

    let mut app = App { buffer: binary };
    app.run().expect("running app");
}
