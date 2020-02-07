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
use itertools::Itertools;
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

            // Render binary hex text
            let info_style = Style::default().fg(Color::White);
            let hex_text = [
                Text::styled(format!("Hello world"), info_style),
                Text::styled(format!("this is a test"), info_style),
            ];
            Paragraph::new(hex_text.iter())
                .block(Block::default().title("Binary").borders(Borders::ALL))
                .wrap(true)
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

fn format_binary(data: &[u8]) -> Result<String> {
    let sections: String = data
        .iter()
        .map(|x| format!("{:02x?}", x))
        .fold(Vec::new(), |mut acc, v| match acc.last_mut() {
            None => {
                acc.push(vec![v]);
                acc
            }
            Some(a) => {
                if a.len() == 1 {
                    a.push(v);
                    acc
                } else {
                    acc.push(vec![v]);
                    acc
                }
            }
        })
        .iter()
        .map(|pair| pair.join(""))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(sections)
}

fn split_binary_data(data: &[u8]) -> itertools::IntoChunks<std::slice::Iter<'_, u8>> {
    data.into_iter().chunks(16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_representation() {
        let data = b"\x7f\x45\x4c\x46\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let expected = "7f45 4c46 0201 0100 0000 0000 0000 0000".to_string();

        assert_eq!(format_binary(data).unwrap(), expected);
    }

    #[test]
    fn test_split_binary_data() {
        let data = b"\x7f\x45\x4c\x46\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03\x00\x3e\x00\x01\x00\x00\x00\x30\x11\x04\x00\x00\x00\x00\x00";
        let expected = vec![
            b"\x7f\x45\x4c\x46\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            b"\x03\x00\x3e\x00\x01\x00\x00\x00\x30\x11\x04\x00\x00\x00\x00\x00",
        ];
        assert_eq!(
            split_binary_data(data)
                .into_iter()
                .map(|c| c.collect::<Vec<_>>())
                .copied()
                .collect::<Vec<_>>(),
            expected
        );
    }
}
