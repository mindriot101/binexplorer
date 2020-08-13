#![warn(rust_2018_idioms)]
use std::fmt::Write;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
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

        let quit = Arc::new(Mutex::new(false));

        let quitter = quit.clone();
        thread::spawn(move || {
            let stdin = io::stdin();
            for event in stdin.keys() {
                if let Ok(_) = event {
                    let mut quit = quitter.lock().unwrap();
                    *quit = true;
                }
            }
        });

        loop {
            let quit = quit.lock().unwrap();
            if *quit {
                break;
            }
            drop(quit);

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
                    .wrap(Wrap { trim: true });
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
