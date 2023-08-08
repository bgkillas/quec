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
'y' to copy line\n\
'd' to cut line\n\
'p' to print line\n\
'w' to save\n\
'q' to quit\n\
'u'/'z' to undo\n\
'x' to redo\n\
'/' to start search mode\n\
search mode:\n\
'esc' to quit search mode\n\
'enter' to search through file"
    );
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
pub fn clear(lines: &[Vec<char>], top: usize, height: usize)
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
            .map(|vec| vec.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("\n")
            .replace('\t', " ")
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