use crate::history::History;
use console::{Key, Term};
use std::{
    cmp::{min, Ordering},
    fs::{canonicalize, File},
    io::{BufRead, BufReader, Read, Stdout, Write},
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
pub struct Files
{
    pub lines: Vec<Vec<char>>,
    pub history: History,
    pub save_file_path: String,
    pub history_file: String,
    pub placement: usize,
    pub line: usize,
    pub start: usize,
    pub top: usize,
    pub cursor: usize,
}
pub fn save_file(files: &mut Files, history_dir: String) -> String
{
    files.lines = files
        .lines
        .iter()
        .map(|line| line.iter().collect::<String>().trim_end().chars().collect())
        .collect();
    let mut result: Vec<u8> = files
        .lines
        .iter()
        .map(|line| line.iter().collect::<String>().as_bytes().to_vec())
        .flat_map(|mut line| {
            line.push(b'\n');
            line.into_iter()
        })
        .collect();
    result.pop();
    while let Some(last) = result.last()
    {
        if *last == b'\n'
        {
            result.pop();
            files.lines.pop();
        }
        else
        {
            break;
        }
    }
    result.push(b'\n');
    let mut err = String::new();
    match File::create(files.save_file_path.clone())
    {
        Ok(mut file) =>
        {
            file.write_all(&result).unwrap();
            loop
            {
                if !files.history.list.is_empty()
                    && (
                        files.history.list[0].add,
                        files.history.list[0].split,
                        files.history.list[0].line.clone(),
                    ) == (true, true, None)
                    && files.history.list[0].pos.0 >= files.lines.len()
                {
                    files.history.list.remove(0);
                }
                else
                {
                    break;
                }
            }
            if !files.history.list.is_empty()
            {
                if files.history_file.is_empty()
                {
                    files.history_file =
                        get_file(files.save_file_path.clone(), history_dir.clone());
                }
                File::create(files.history_file.clone())
                    .unwrap()
                    .write_all(&files.history.to_bytes())
                    .unwrap();
            }
            else
            {
                let _ = std::fs::remove_file(files.history_file.clone());
            }
        }
        Err(e) => err = e.to_string(),
    }
    err
}
pub fn open_file(file: String, history_dir: String) -> Files
{
    let mut history_file = String::new();
    let (mut lines, history) = if File::open(file.clone()).is_err()
    {
        (
            Vec::new(),
            History {
                pos: 0,
                list: Vec::new(),
            },
        )
    }
    else
    {
        let f = BufReader::new(File::open(&file).unwrap())
            .lines()
            .map(|l| {
                l.unwrap()
                    .chars()
                    .filter(|c| {
                        !c.is_ascii()
                            || c.is_ascii_graphic()
                            || c == &' '
                            || c == &'\t'
                            || c == &'\n'
                    })
                    .collect::<Vec<char>>()
            })
            .collect::<Vec<Vec<char>>>();
        history_file = get_file(file.clone(), history_dir.clone());
        (
            f,
            if let Ok(mut f) = File::open(history_file.clone())
            {
                let mut read_bytes = Vec::new();
                f.read_to_end(&mut read_bytes).unwrap();
                History::from_bytes(&read_bytes)
            }
            else
            {
                History {
                    pos: 0,
                    list: Vec::new(),
                }
            },
        )
    };
    if lines.is_empty()
    {
        lines.push(Vec::new());
    }
    Files {
        lines,
        history,
        save_file_path: file,
        history_file,
        placement: 0,
        line: 0,
        top: 0,
        start: 0,
        cursor: 0,
    }
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
        (size.ws_row as usize - 1, size.ws_col as usize)
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
        lines[line][placement..min(lines[line].len(), width + start)]
            .iter()
            .collect::<String>()
            .replace('\t', " ")
    );
}
pub fn clear(lines: &[Vec<char>], top: usize, height: usize, start: usize, width: usize)
{
    print!(
        "\x1B[H\x1B[J{}",
        lines[top..min(lines.len(), height + top)]
            .iter()
            .map(|vec| {
                if start <= vec.len()
                {
                    vec[start..min(vec.len(), width + start)]
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
pub fn get_word(term: &Term, stdout: &mut Stdout, height: usize) -> Result<String, ()>
{
    let mut index = 0;
    let mut file_path = Vec::new();
    loop
    {
        let c = read_single_char(term);
        match c
        {
            '\x1C' if index != file_path.len() =>
            {
                //right
                index += 1;
            }
            '\x1B' if index != 0 =>
            {
                //left
                index -= 1;
            }
            '\x08' if index != 0 =>
            {
                file_path.remove(index - 1);
                index = index.saturating_sub(1);
            }
            '\x1A' =>
            {
                return Err(());
            }
            '\n' => break,
            '\0' =>
            {}
            _ if !c.is_ascii() || c.is_ascii_graphic() || c == ' ' || c == '\t' || c == '\n' =>
            {
                file_path.insert(index, c);
                index += 1;
            }
            _ =>
            {}
        }
        print!(
            "\x1B[H\x1B[{}B\x1B[K{}\x1B[G{}",
            height + 1,
            file_path.iter().collect::<String>(),
            if index == 0
            {
                String::new()
            }
            else
            {
                "\x1B[".to_owned() + &index.to_string() + "C"
            },
        );
        stdout.flush().unwrap();
    }
    Ok(file_path.iter().collect())
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
        "\x1B[H\x1B[{}B\x1B[K{} \x1B[{}C{}\x1B[H{}{}",
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