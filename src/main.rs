use std::convert::TryFrom;
use std::fs::File;
use std::io::{stdout, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Error, Result};
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
    backend::{Backend, CrosstermBackend},
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

#[derive(Debug)]
enum ParseChar {
    I8,
    U8,
    Bool,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
}

impl TryFrom<char> for ParseChar {
    type Error = Error;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            'b' => Ok(ParseChar::I8),
            'B' => Ok(ParseChar::U8),
            '?' => Ok(ParseChar::Bool),
            'h' => Ok(ParseChar::I16),
            'H' => Ok(ParseChar::U16),
            'i' => Ok(ParseChar::I32),
            'I' => Ok(ParseChar::U32),
            'l' => Ok(ParseChar::I64),
            'L' => Ok(ParseChar::U64),
            _ => Err(anyhow!("invalid char {}", c)),
        }
    }
}

struct BinExplorer {
    instructions: String,
    should_quit: bool,
}

impl BinExplorer {
    fn new() -> Self {
        Self {
            instructions: String::new(),
            should_quit: false,
        }
    }

    fn handle_key(&mut self, key: char) {
        if let Ok(ins) = ParseChar::try_from(key) {
            todo!("{:?}", ins);
        }
    }

    fn render_parsed<B: Backend>(
        &mut self,
        mut f: &mut tui::terminal::Frame<'_, B>,
        chunk: tui::layout::Rect,
    ) {
        SelectableList::default()
            .block(Block::default().title("Output").borders(Borders::ALL))
            .items(&["a", "b", "c"])
            .select(Some(1))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().modifier(Modifier::ITALIC))
            .highlight_symbol(">>")
            .render(&mut f, chunk);
    }

    // fn render_editor(&mut self) {
    //     Block::default()
    //         .title("Editor")
    //         .borders(Borders::ALL)
    //         .render(&mut f, chunks[2]);
    // }
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

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || loop {
        if event::poll(Duration::from_millis(250)).unwrap() {
            if let CEvent::Key(key) = event::read().unwrap() {
                tx.send(Event::Input(key)).unwrap();
            }
        }

        tx.send(Event::Tick).unwrap();
    });

    let mut app = BinExplorer::new();

    terminal.clear()?;

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

            app.render_parsed(&mut f, chunks[1]);
            // app.render_editor(&mut f, chunks[2]);
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                // Supported keys:
                //
                // - q: quit
                // - b: i8
                // - B: u8
                // - ?: bool
                // - h: i16
                // - H: u16
                // - i: i32
                // - I: u32
                // - l: i64
                // - L: u64
                KeyCode::Char('q') => {
                    // TODO: why does this not work? execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char(c) => app.handle_key(c),
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

fn _split_binary_data(data: &[u8]) -> itertools::IntoChunks<std::slice::Iter<'_, u8>> {
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

    // #[test]
    // #[ignore]
    // fn test_split_binary_data() {
    //     let data = b"\x7f\x45\x4c\x46\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03\x00\x3e\x00\x01\x00\x00\x00\x30\x11\x04\x00\x00\x00\x00\x00";
    //     let expected = vec![
    //         b"\x7f\x45\x4c\x46\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00",
    //         b"\x03\x00\x3e\x00\x01\x00\x00\x00\x30\x11\x04\x00\x00\x00\x00\x00",
    //     ];
    //     assert_eq!(
    //         split_binary_data(data)
    //             .into_iter()
    //             .map(|c| c.collect::<Vec<_>>())
    //             .copied()
    //             .collect::<Vec<_>>(),
    //         expected
    //     );
    // }
}
