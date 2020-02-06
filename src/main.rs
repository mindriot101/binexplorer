use std::fs::File;
use std::io::{stdout, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use structopt::StructOpt;
use tui::style::{Color, Modifier, Style};
use tui::widgets::*;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};

#[derive(StructOpt, Debug)]
struct Opts {
    #[structopt(parse(from_os_str))]
    filename: PathBuf,
}

enum Event<I> {
    Input(I),
    Tick,
}

/// RAII wrapper around setting raw mode
struct RawMode;

impl RawMode {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        Ok(RawMode {})
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        disable_raw_mode().expect("disabling raw mode");
    }
}

fn xxd_view<'t, T>(buf: &'t [u8]) -> Paragraph<T>
where
    T: std::iter::Iterator<Item = &'t Text<'t>>,
{
    // Render binary hex text
    let info_style = Style::default().fg(Color::White);
    let hex_text = [
        Text::styled(format!("Hello world"), info_style),
        Text::styled(format!("this is a test"), info_style),
    ];
    Paragraph::new(hex_text.iter())
}

fn main() -> Result<()> {
    let opts = Opts::from_args();

    let mut file = File::open(&opts.filename)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let _raw = RawMode::new()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    terminal.clear()?;

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || loop {
        if event::poll(Duration::from_millis(250)).unwrap() {
            if let CEvent::Key(key) = event::read().unwrap() {
                tx.send(Event::Input(key)).unwrap();
            }
        }

        tx.send(Event::Tick).unwrap();
    });

    loop {
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(33),
                        Constraint::Percentage(33),
                        Constraint::Percentage(33),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let left_pane = xxd_view(&buf);
            left_pane
                .block(Block::default().title("Binary").borders(Borders::ALL))
                .render(&mut f, chunks[0]);
            SelectableList::default()
                .block(Block::default().title("Output").borders(Borders::ALL))
                .items(&["a", "b", "c"])
                .select(Some(1))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().modifier(Modifier::ITALIC))
                .highlight_symbol(">>")
                .render(&mut f, chunks[1]);
            Block::default()
                .title("Editor")
                .borders(Borders::ALL)
                .render(&mut f, chunks[2]);
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;
                    break;
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }

    disable_raw_mode()?;

    Ok(())
}
