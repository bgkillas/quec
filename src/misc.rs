use crate::history::History;
use console::{Key, Term};
use std::{cmp::Ordering, fs::canonicalize};
#[cfg(not(unix))]
use term_size::dimensions;
#[cfg(unix)]
use {
    libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ},
    std::mem,
};
pub fn help()
{
    println!(
        "'i' to enter edit mode\n\
'esc' to exit edit mode\n\
'h' left\n\
'l' right\n\
'j' down\n\
'k' up\n\
'0' move to beginning of line\n\
'$' move to end of line\n\
'page down' move 1 page down\n\
'page up' move 1 page up\n\
'home' go to start of file\n\
'end' go to end of file\n\
'y' to copy line\n\
'd' to cut line\n\
'p' to print line\n\
'w' to save\n\
'`' to go to next file\n\
'~' to go to last file\n\
'q' to quit\n\
'u' to undo\n\
'U' to redo\n\
'/' to start search mode\n\
search mode:\n\
'esc' to quit search mode\n\
'enter' to search through file"
    );
}
pub struct Files
{
    pub lines: Vec<Vec<char>>,
    pub history: History,
    pub save_file: String,
    pub history_file: String,
    pub placement: usize,
    pub line: usize,
    pub start: usize,
    pub top: usize,
    pub cursor: usize,
}
pub fn fix_history(history: &mut History)
{
    if history.pos != 0
    {
        history.list.drain(..history.pos);
        history.pos = 0;
    }
}
pub fn fix_top(top: usize, line: usize, height: usize) -> usize
{
    match top.cmp(&line)
    {
        Ordering::Greater => line,
        Ordering::Less =>
        {
            if height > line
            {
                0
            }
            else if top + height > line
            {
                top
            }
            else
            {
                line - height + 1
            }
        }
        Ordering::Equal => top,
    }
}
pub fn read_single_char(term: &Term) -> char
{
    match term.read_key().unwrap()
    {
        Key::Char(c) => c,
        Key::Enter => '\n',
        Key::Backspace => '\x08',
        Key::ArrowLeft => '\x1B',
        Key::Home => '\x01',
        Key::End => '\x02',
        Key::PageUp => '\x03',
        Key::PageDown => '\x04',
        Key::ArrowRight => '\x1C',
        Key::ArrowUp => '\x1D',
        Key::ArrowDown => '\x1E',
        Key::Escape => '\x1A',
        Key::Tab => '\t',
        _ => '\0',
    }
}
#[cfg(unix)]
pub fn get_dimensions() -> (usize, usize)
{
    unsafe {
        let mut size: winsize = mem::zeroed();
        ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut size);
        (size.ws_row as usize, size.ws_col as usize)
    }
}
#[cfg(not(unix))]
pub fn get_dimensions() -> (usize, usize)
{
    if let Some((width, height)) = dimensions()
    {
        (height, width)
    }
    else
    {
        (80, 80)
    }
}
pub fn clear_line(lines: &[Vec<char>], line: usize, placement: usize, width: usize, start: usize)
{
    print!(
        "\x1B[K{}",
        lines[line][placement
            ..if lines[line].len() < width + start
            {
                lines[line].len()
            }
            else
            {
                width + start
            }]
            .iter()
            .collect::<String>()
            .replace('\t', " ")
    );
}
pub fn clear(lines: &[Vec<char>], top: usize, height: usize, start: usize, width: usize)
{
    print!(
        "\x1B[H\x1B[J{}",
        lines[top..if lines.len() < height + top
        {
            lines.len()
        }
        else
        {
            height + top
        }]
            .iter()
            .map(|vec| {
                if start <= vec.len()
                {
                    vec[start
                        ..if vec.len() < width + start
                        {
                            vec.len()
                        }
                        else
                        {
                            width + start
                        }]
                        .iter()
                        .collect::<String>()
                }
                else
                {
                    "".to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
            .replace('\t', " ")
    );
}
pub fn print_line_number(
    height: usize,
    width: usize,
    line: usize,
    placement: usize,
    top: usize,
    start: usize,
)
{
    print!(
        "\x1B[G\x1B[{}B\x1B[{}C\x1B[K{},{}\x1B[H{}{}",
        height,
        width - 15,
        line + 1,
        placement + 1,
        if line - top == 0
        {
            String::new()
        }
        else
        {
            "\x1B[".to_owned() + &(line - top).to_string() + "B"
        },
        if placement - start == 0
        {
            String::new()
        }
        else
        {
            "\x1B[".to_owned() + &(placement - start).to_string() + "C"
        }
    );
}
#[cfg(unix)]
pub fn get_file(path: String, history_dir: String) -> String
{
    history_dir.clone()
        + &canonicalize(path)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('/', "%")
}
#[cfg(not(unix))]
pub fn get_file(path: String, history_dir: String) -> String
{
    let history_file = canonicalize(path)
        .unwrap()
        .to_str()
        .unwrap()
        .replace('\\', "%");
    history_dir.clone() + &history_file[history_file.find(':').unwrap() + 1..]
}