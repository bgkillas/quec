use crate::history::History;
use crossterm::{
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use std::{
    cmp::{min, Ordering},
    fs::canonicalize,
    io::{stdout, Write},
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
pub fn read_single_char() -> char
{
    let result = match match read()
    {
        Ok(c) => c,
        Err(_) => return '\0',
    }
    {
        Event::Key(KeyEvent {
            code, modifiers, ..
        }) => match (code, modifiers)
        {
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => c,
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => '\x14',
            (KeyCode::Esc, KeyModifiers::NONE) => '\x1A',
            (KeyCode::Enter, KeyModifiers::NONE) => '\n',
            (KeyCode::Backspace, KeyModifiers::NONE) => '\x08',
            (KeyCode::Left, KeyModifiers::NONE) => '\x1B',
            (KeyCode::Right, KeyModifiers::NONE) => '\x1C',
            (KeyCode::Left, KeyModifiers::ALT) => '\x12',
            (KeyCode::Right, KeyModifiers::ALT) => '\x13',
            (KeyCode::Up, KeyModifiers::NONE) => '\x1D',
            (KeyCode::Down, KeyModifiers::NONE) => '\x1E',
            (KeyCode::PageDown, KeyModifiers::NONE) => '\x04',
            (KeyCode::PageUp, KeyModifiers::NONE) => '\x03',
            (KeyCode::End, KeyModifiers::NONE) => '\x02',
            (KeyCode::Home, KeyModifiers::NONE) => '\x01',
            (KeyCode::Tab, KeyModifiers::NONE) => '\t',
            _ => '\0',
        },
        _ => '\0',
    };
    if result == '\x14'
    {
        print!("\x1b[G\x1b[{}B\x1b[?1049l", get_dimensions().0);
        stdout().flush().unwrap();
        terminal::disable_raw_mode().unwrap();
        std::process::exit(130);
    }
    result
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
            .join("\x1b[E\x1b[G")
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