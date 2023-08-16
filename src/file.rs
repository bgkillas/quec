use crate::{
    history::History,
    misc::{get_file, read_single_char},
};
use console::Term;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Stdout, Write},
};
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
            .map(|l| match l
            {
                Ok(l) => l
                    .chars()
                    .filter(|c| {
                        !c.is_ascii()
                            || c.is_ascii_graphic()
                            || c == &' '
                            || c == &'\t'
                            || c == &'\n'
                    })
                    .collect::<Vec<char>>(),
                Err(e) =>
                {
                    println!("\x1b[?1049l{}", e);
                    std::process::exit(1);
                }
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
            '\x1b' if index != 0 =>
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
            "\x1b[H\x1b[{}B\x1b[K{}\x1b[G{}",
            height + 1,
            file_path.iter().collect::<String>(),
            if index == 0
            {
                String::new()
            }
            else
            {
                "\x1b[".to_owned() + &index.to_string() + "C"
            },
        );
        stdout.flush().unwrap();
    }
    Ok(file_path.iter().collect())
}