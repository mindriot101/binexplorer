use std::convert::TryFrom;
use std::fs::File;
use std::io::{stdout, Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Error, Result};
use byteorder::{NativeEndian, ReadBytesExt};
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
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
    // Tick,
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

impl ParseChar {
    fn take_from<R>(&self, mut buf: R) -> Result<String>
    where
        R: ReadBytesExt,
    {
        match *self {
            ParseChar::I8 => Ok(format!("{}", buf.read_i8()?)),
            ParseChar::U8 => Ok(format!("{}", buf.read_u8()?)),
            ParseChar::Bool => Ok(format!("{}", buf.read_u8()?)),
            ParseChar::I16 => Ok(format!("{}", buf.read_i16::<NativeEndian>()?)),
            ParseChar::U16 => Ok(format!("{}", buf.read_u16::<NativeEndian>()?)),
            ParseChar::I32 => Ok(format!("{}", buf.read_i32::<NativeEndian>()?)),
            ParseChar::U32 => Ok(format!("{}", buf.read_u32::<NativeEndian>()?)),
            ParseChar::I64 => Ok(format!("{}", buf.read_i64::<NativeEndian>()?)),
            ParseChar::U64 => Ok(format!("{}", buf.read_u64::<NativeEndian>()?)),
            _ => Ok("".to_string()),
        }
    }
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

impl From<&ParseChar> for char {
    fn from(pc: &ParseChar) -> char {
        match pc {
            ParseChar::I8 => 'b',
            ParseChar::U8 => 'B',
            ParseChar::Bool => '?',
            ParseChar::I16 => 'h',
            ParseChar::U16 => 'H',
            ParseChar::I32 => 'i',
            ParseChar::U32 => 'I',
            ParseChar::I64 => 'l',
            ParseChar::U64 => 'L',
        }
    }
}

#[derive(Debug)]
struct BinExplorer<'a> {
    buffer: &'a [u8],
    instructions: Vec<ParseChar>,
    should_quit: bool,
}

impl<'a> BinExplorer<'a> {
    fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            instructions: Vec::new(),
            should_quit: false,
        }
    }

    fn handle_key(&mut self, key: char) {
        log::debug!("key {} pressed", key);
        if let Ok(ins) = ParseChar::try_from(key) {
            log::debug!("key parse ok: {:?}", ins);
            self.instructions.push(ins);
        }
    }

    fn handle_backspace(&mut self) {
        log::debug!("backspace pressed");
        if !self.instructions.is_empty() {
            self.instructions.pop();
        }
    }

    fn render_raw<B: Backend>(
        &mut self,
        mut f: &mut tui::terminal::Frame<'_, B>,
        chunk: tui::layout::Rect,
    ) {
        let nlines = chunk.height;
        // Render binary hex text
        let hex_text: Vec<_> = self
            .buffer
            .chunks(16)
            .map(|c| {
                let formatted = format_binary(c);
                Text::raw(formatted)
            })
            .take(nlines as usize)
            .collect();

        Paragraph::new(hex_text.iter())
            .block(Block::default().title("Binary").borders(Borders::ALL))
            .wrap(true)
            .render(&mut f, chunk);
    }

    fn render_parsed<B: Backend>(
        &mut self,
        mut f: &mut tui::terminal::Frame<'_, B>,
        chunk: tui::layout::Rect,
    ) {
        let s = self.parsed_string();
        log::debug!("rendering parsed string: {:?}", s);
        let text = [Text::raw(s)];

        Paragraph::new(text.iter())
            .wrap(true)
            .block(Block::default().title("Output").borders(Borders::ALL))
            .render(&mut f, chunk);
    }

    fn render_editor<B: Backend>(
        &mut self,
        mut f: &mut tui::terminal::Frame<'_, B>,
        chunk: tui::layout::Rect,
    ) {
        let s = self.instructions_string();
        log::debug!("rendering instructions string: {:?}", s);
        let text = [Text::raw(s)];

        Paragraph::new(text.iter())
            .wrap(true)
            .block(Block::default().title("Editor").borders(Borders::ALL))
            .render(&mut f, chunk);
    }

    fn parsed_string(&self) -> String {
        let mut cursor = Cursor::new(self.buffer);
        self.instructions
            .iter()
            .map(|i| i.take_from(&mut cursor).unwrap())
            .collect::<Vec<String>>()
            .join(" ")
    }

    fn instructions_string(&self) -> String {
        self.instructions.iter().map(|p| char::from(p)).collect()
    }
}

fn main() -> Result<()> {
    // Set up logging
    let requests = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .append(false)
        .build("binexplorer.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("requests", Box::new(requests)))
        .logger(Logger::builder().build("binexplorer", LevelFilter::Debug))
        .build(
            Root::builder()
                .appender("requests")
                .build(LevelFilter::Warn),
        )
        .unwrap();

    let _handle = log4rs::init_config(config).unwrap();

    log::info!("logging set up");

    let opts = Opts::from_args();

    let mut file = File::open(&opts.filename)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let _raw = RawMode::new()?;
    terminal.hide_cursor()?;

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || loop {
        if event::poll(Duration::from_millis(250)).unwrap() {
            if let CEvent::Key(key) = event::read().unwrap() {
                tx.send(Event::Input(key)).unwrap();
            }
        }

        // tx.send(Event::Tick).unwrap();
    });

    let mut app = BinExplorer::new(&buf);

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

            app.render_raw(&mut f, chunks[0]);
            app.render_parsed(&mut f, chunks[1]);
            app.render_editor(&mut f, chunks[2]);
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    // TODO: why does this not work? execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char(c) => app.handle_key(c),
                KeyCode::Backspace => app.handle_backspace(),
                _ => {}
            },
            // Event::Tick => {}
        }
    }

    Ok(())
}

fn format_binary(data: &[u8]) -> String {
    let mut sections: String = data
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
    sections.push_str("\n");
    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_representation() {
        let data = b"\x7f\x45\x4c\x46\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let expected = "7f45 4c46 0201 0100 0000 0000 0000 0000\n".to_string();

        assert_eq!(format_binary(data), expected);
    }
}
