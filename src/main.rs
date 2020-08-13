// use std::fs::File;
// use std::io::{stdout, BufReader, Cursor, Read, Write};
// use std::path::PathBuf;
// use std::sync::mpsc;
// use std::thread;
// use std::time::Duration;

// use anyhow::{anyhow, Result};
// use byteorder::{NativeEndian, ReadBytesExt};
// use crossterm::{
//     event::{self, Event as CEvent, KeyCode},
//     execute,
//     terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
// };
// use log::LevelFilter;
// use log4rs::append::file::FileAppender;
// use log4rs::config::{Appender, Config, Logger, Root};
// use log4rs::encode::pattern::PatternEncoder;
// use structopt::StructOpt;
// use tui::widgets::*;
// use tui::{
//     backend::{self, CrosstermBackend},
//     layout::{Constraint, Direction, Layout},
//     Terminal,
// };

// mod parsing;
// mod presentation;

// #[derive(StructOpt, Debug)]
// struct Opts {
//     #[structopt(parse(from_os_str))]
//     filename: PathBuf,
// }

// enum Event<I> {
//     Input(I),
// }

// /// RAII wrapper around setting raw mode
// struct RawMode;

// impl RawMode {
//     fn new() -> Result<Self> {
//         enable_raw_mode()?;
//         Ok(RawMode {})
//     }
// }

// impl Drop for RawMode {
//     fn drop(&mut self) {
//         disable_raw_mode().expect("disabling raw mode");
//     }
// }

// #[derive(Debug, PartialEq, Eq, Clone, Copy)]
// enum ParseChar {
//     I8,
//     U8,
//     // Bool,
//     I16,
//     U16,
//     // I32,
//     // U32,
//     // I64,
//     // U64,
// }

// impl From<&ParseChar> for char {
//     fn from(pc: &ParseChar) -> char {
//         match pc {
//             ParseChar::I8 => 'b',
//             ParseChar::U8 => 'B',
//             // ParseChar::Bool => '?',
//             ParseChar::I16 => 'h',
//             ParseChar::U16 => 'H',
//             // ParseChar::I32 => 'i',
//             // ParseChar::U32 => 'I',
//             // ParseChar::I64 => 'l',
//             // ParseChar::U64 => 'L',
//         }
//     }
// }

// #[derive(Debug, PartialEq, Eq, Clone, Copy)]
// struct MultipleParseChar {
//     count: usize,
//     c: ParseChar,
// }

// impl MultipleParseChar {
//     fn single(c: ParseChar) -> Self {
//         Self { count: 1, c }
//     }

//     fn many(c: ParseChar, count: usize) -> Self {
//         Self { count, c }
//     }

//     fn to_str(&self) -> String {
//         if self.count > 1 {
//             format!("{}{}", self.count, char::from(&self.c))
//         } else {
//             format!("{}", char::from(&self.c))
//         }
//     }
// }

// impl MultipleParseChar {
//     fn take_from<R>(&self, mut buf: R) -> Result<String>
//     where
//         R: ReadBytesExt,
//     {
//         match self.c {
//             ParseChar::I8 => {
//                 let mut out = vec![0i8; self.count];
//                 buf.read_i8_into(&mut out)?;
//                 Ok(out
//                     .iter()
//                     .map(|c| format!("{}", c))
//                     .collect::<Vec<_>>()
//                     .join(" "))
//             }
//             ParseChar::U8 => {
//                 let mut out = vec![0u8; self.count];
//                 let n = buf.read(&mut out)?;
//                 if n != self.count {
//                     return Err(anyhow!("not enough bytes read, {} != {}", n, self.count));
//                 }
//                 Ok(out
//                     .iter()
//                     .map(|c| format!("{}", c))
//                     .collect::<Vec<_>>()
//                     .join(" "))
//             }
//             ParseChar::I16 => {
//                 let mut out = vec![0i16; self.count];
//                 buf.read_i16_into::<NativeEndian>(&mut out)?;
//                 Ok(out
//                     .iter()
//                     .map(|c| format!("{}", c))
//                     .collect::<Vec<_>>()
//                     .join(" "))
//             }
//             ParseChar::U16 => {
//                 let mut out = vec![0u16; self.count];
//                 buf.read_u16_into::<NativeEndian>(&mut out)?;
//                 Ok(out
//                     .iter()
//                     .map(|c| format!("{}", c))
//                     .collect::<Vec<_>>()
//                     .join(" "))
//             }
//         }
//     }
// }

// #[derive(Debug)]
// struct BinExplorer<'a> {
//     buffer: &'a [u8],
//     instructions: Vec<MultipleParseChar>,
//     raw_instructions: String,
//     should_quit: bool,
// }

// impl<'a> BinExplorer<'a> {
//     fn new(buffer: &'a [u8]) -> Self {
//         Self {
//             buffer,
//             instructions: Vec::new(),
//             should_quit: false,
//             raw_instructions: String::new(),
//         }
//     }

//     fn handle_key(&mut self, key: char) {
//         log::debug!("key {} pressed", key);
//         self.raw_instructions.push(key);
//         let instructions = parsing::parse_input(&self.raw_instructions).unwrap();
//         self.instructions = instructions;
//     }

//     fn handle_backspace(&mut self) {
//         log::debug!("backspace pressed");
//         if !self.instructions.is_empty() {
//             self.instructions.pop();
//         }
//         self.raw_instructions = self
//             .instructions
//             .iter()
//             .map(MultipleParseChar::to_str)
//             .collect();
//     }

//     fn render_raw<B: backend::Backend>(
//         &mut self,
//         mut f: &mut tui::terminal::Frame<'_, B>,
//         chunk: tui::layout::Rect,
//     ) {
//         // let mut reader: Box<dyn Read> = Box::new(File::open("target/release/binexplorer").unwrap());
//         let file = File::open("target/release/binexplorer").unwrap();
//         let reader = BufReader::new(file);

//         // TODO: output the text to this buffer
//         let mut out = Vec::new();
//         presentation::write_formatted_binary(reader, 16, &mut out).unwrap();

//         let hex_text = [Text::raw(String::from_utf8(out).unwrap())];

//         Paragraph::new(hex_text.iter())
//             .block(Block::default().title("Binary").borders(Borders::ALL))
//             .wrap(true)
//             .render(&mut f, chunk);
//     }

//     fn render_parsed<B: backend::Backend>(
//         &mut self,
//         mut f: &mut tui::terminal::Frame<'_, B>,
//         chunk: tui::layout::Rect,
//     ) {
//         let s = self.parsed_string();
//         log::debug!("rendering parsed string: {:?}", s);
//         let text = [Text::raw(s)];

//         Paragraph::new(text.iter())
//             .wrap(true)
//             .block(Block::default().title("Output").borders(Borders::ALL))
//             .render(&mut f, chunk);
//     }

//     fn render_editor<B: backend::Backend>(
//         &mut self,
//         mut f: &mut tui::terminal::Frame<'_, B>,
//         chunk: tui::layout::Rect,
//     ) {
//         let s = &self.raw_instructions;
//         log::debug!("rendering instructions string: {:?}", s);
//         let text = [Text::raw(s)];

//         Paragraph::new(text.iter())
//             .wrap(true)
//             .block(Block::default().title("Editor").borders(Borders::ALL))
//             .render(&mut f, chunk);
//     }

//     fn parsed_string(&self) -> String {
//         let mut cursor = Cursor::new(self.buffer);
//         self.instructions
//             .iter()
//             .map(|i| i.take_from(&mut cursor).unwrap())
//             .collect::<Vec<String>>()
//             .join(" ")
//     }
// }

// fn main() -> Result<()> {
//     // Set up logging
//     let requests = FileAppender::builder()
//         .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
//         .append(false)
//         .build("binexplorer.log")
//         .unwrap();

//     let config = Config::builder()
//         .appender(Appender::builder().build("requests", Box::new(requests)))
//         .logger(Logger::builder().build("binexplorer", LevelFilter::Debug))
//         .build(
//             Root::builder()
//                 .appender("requests")
//                 .build(LevelFilter::Warn),
//         )
//         .unwrap();

//     let _handle = log4rs::init_config(config).unwrap();

//     log::info!("logging set up");

//     let opts = Opts::from_args();

//     let mut file = File::open(&opts.filename)?;
//     let mut buf = Vec::new();
//     file.read_to_end(&mut buf)?;

//     let mut stdout = stdout();
//     execute!(stdout, EnterAlternateScreen)?;

//     let backend = CrosstermBackend::new(stdout);
//     let mut terminal = Terminal::new(backend)?;

//     let _raw = RawMode::new()?;
//     terminal.hide_cursor()?;

//     let (tx, rx) = mpsc::channel();

//     thread::spawn(move || loop {
//         if event::poll(Duration::from_millis(250)).unwrap() {
//             if let CEvent::Key(key) = event::read().unwrap() {
//                 tx.send(Event::Input(key)).unwrap();
//             }
//         }
//     });

//     let mut app = BinExplorer::new(&buf);

//     terminal.clear()?;

//     loop {
//         terminal.draw(|mut f| {
//             let chunks = Layout::default()
//                 .direction(Direction::Horizontal)
//                 .margin(1)
//                 .constraints(
//                     [
//                         Constraint::Percentage(33),
//                         Constraint::Percentage(33),
//                         Constraint::Percentage(33),
//                     ]
//                     .as_ref(),
//                 )
//                 .split(f.size());

//             app.render_raw(&mut f, chunks[0]);
//             app.render_parsed(&mut f, chunks[1]);
//             app.render_editor(&mut f, chunks[2]);
//         })?;

//         match rx.recv()? {
//             Event::Input(event) => match event.code {
//                 KeyCode::Char('q') => {
//                     execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
//                     terminal.show_cursor()?;
//                     break;
//                 }
//                 KeyCode::Char(c) => app.handle_key(c),
//                 KeyCode::Backspace => app.handle_backspace(),
//                 _ => {}
//             },
//         }
//     }

//     Ok(())
// }

// /*
// fn parse(input: &str) -> IResult<&str, Vec<MultipleParseChar>> {
//     many0(parse_multiple)(input)
// }

// fn parse_i8(input: &str) -> IResult<&str, ParseChar> {
//     let (input, _) = tag("b")(input)?;
//     Ok((input, ParseChar::I8))
// }

// fn parse_u8(input: &str) -> IResult<&str, ParseChar> {
//     let (input, _) = tag("B")(input)?;
//     Ok((input, ParseChar::U8))
// }

// fn parse_i16(input: &str) -> IResult<&str, ParseChar> {
//     let (input, _) = tag("h")(input)?;
//     Ok((input, ParseChar::I16))
// }

// fn parse_u16(input: &str) -> IResult<&str, ParseChar> {
//     let (input, _) = tag("H")(input)?;
//     Ok((input, ParseChar::U16))
// }

// fn parse_multiple(input: &str) -> IResult<&str, MultipleParseChar> {
//     let (input, n_txt) = complete::digit0(input)?;
//     let (input, pc) = alt((parse_i8, parse_u8, parse_i16, parse_u16))(input)?;

//     if let Ok(n) = n_txt.parse() {
//         Ok((input, MultipleParseChar::many(pc, n)))
//     } else {
//         Ok((input, MultipleParseChar::single(pc)))
//     }
// }
// */
// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_binary_representation() {
//         let data = b"\x7f\x45\x4c\x46\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00";
//         let expected = "7f45 4c46 0201 0100 0000 0000 0000 0000\n".to_string();

//         assert_eq!(format_binary(data), expected);
//     }
// }

fn main() {}
