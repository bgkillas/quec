use crate::history::History;
use console::{Key, Term};
use std::{
    cmp::{min, Ordering},
    fs::canonicalize,
};
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
'y' copy line\n\
'd' cut line\n\
'p' print line\n\
'w' save\n\
's' save as\n\
'o' open file\n\
'`' go to next file\n\
'~' go to last file\n\
'q' quit\n\
'u' undo\n\
'U' redo\n\
'/' search"
    );
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
        Key::ArrowLeft => '\x1b',
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
        (size.ws_row as usize - 1, size.ws_col as usize)
    }
}
#[cfg(not(unix))]
pub fn get_dimensions() -> (usize, usize)
{
    if let Some((width, height)) = dimensions()
    {
        (height - 1, width)
    }
    else
    {
        (80, 80)
    }
}
pub fn clear_line(lines: &[Vec<char>], line: usize, placement: usize, width: usize, start: usize)
{
    print!(
        "\x1b[K{}",
        lines[line][placement..min(lines[line].len(), width + start)]
            .iter()
            .collect::<String>()
            .replace('\t', " ")
    );
}
pub fn clear(lines: &[Vec<char>], top: usize, height: usize, start: usize, width: usize)
{
    print!(
        "\x1b[?25l\x1b[H{}\x1b[?25h",
        lines[top..min(lines.len(), height + top)]
            .iter()
            .map(|vec| {
                if start <= vec.len()
                {
                    vec[start..min(vec.len(), width + start)]
                        .iter()
                        .collect::<String>()
                        + "\x1b[K"
                }
                else
                {
                    "\x1b[K".to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
            .replace('\t', " ")
    );
}
pub fn print_line_number(
    height: usize,
    line: usize,
    placement: usize,
    top: usize,
    start: usize,
    word: String,
)
{
    let n = format!("{},{}", line + 1, placement + 1);
    let i = 10 * (n.len().ilog10() + 1) as usize - n.len();
    print!(
        "\x1b[H\x1b[{}B\x1b[K{} \x1b[{}C{}\x1b[H{}{}",
        height + 1,
        n,
        i,
        word,
        if line - top == 0
        {
            String::new()
        }
        else
        {
            "\x1b[".to_owned() + &(line - top).to_string() + "B"
        },
        if placement - start == 0
        {
            String::new()
        }
        else
        {
            "\x1b[".to_owned() + &(placement - start).to_string() + "C"
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