use console::{Key, Term};
use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};
use std::{
    env::args,
    fs::{canonicalize, create_dir, File},
    io::{stdout, BufRead, BufReader, Read, Write},
    mem,
};
//TODO word wrapping and support files longer then screen
fn main()
{
    let mut args = args().collect::<Vec<String>>();
    args.remove(0);
    let mut debug = false;
    loop
    {
        if args.is_empty()
        {
            return;
        }
        match args[0].as_str()
        {
            "--help" =>
            {
                println!(
                    "'i' to enter edit mode\n\
'esc' to exit edit mode\n\
'd' to cut line\n\
'p' to print line\n\
'w' to save\n\
'q' to quit\n\
'z' to undo\n\
'x' to redo\n\
'/' to start search mode\n\
search mode:\n\
'esc' to quit search mode\n\
'enter' to search through file"
                );
                return;
            }
            "--debug" => debug = true,
            _ => break,
        }
        args.remove(0);
    }
    if args.is_empty()
    {
        return;
    }
    let mut stdout = stdout();
    print!("\x1B[?1049h\x1B[K\x1B[J");
    stdout.flush().unwrap();
    let history_dir = env!("HOME").to_owned() + "/.quec/";
    let _ = create_dir(history_dir.clone());
    let (height, _width) = get_dimensions();
    'outer: for (n, i) in args.iter().enumerate()
    {
        let mut history_file = String::new();
        let (mut lines, mut history) = if File::open(i.clone()).is_err()
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
            let f = BufReader::new(File::open(i.clone()).unwrap())
                .lines()
                .map(|l| {
                    l.unwrap()
                        .chars()
                        .filter(|c| c.is_ascii_graphic() || c == &' ' || c == &'\t' || c == &'\n')
                        .collect::<Vec<char>>()
                })
                .collect::<Vec<Vec<char>>>();
            history_file = history_dir.clone()
                + &canonicalize(i.clone())
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('/', "%");
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
            print!("{}\x1B[H", "\n".repeat(height));
        }
        else if lines.len() > height
        {
            print!(
                "{}\x1B[H",
                lines[..height]
                    .iter()
                    .map(|vec| vec.iter().collect::<String>())
                    .collect::<Vec<String>>()
                    .join("\n")
                    .replace('\t', " "),
            );
        }
        else
        {
            print!(
                "{}{}\x1B[H",
                lines
                    .iter()
                    .map(|vec| vec.iter().collect::<String>())
                    .collect::<Vec<String>>()
                    .join("\n")
                    .replace('\t', " "),
                "\n".repeat(height - lines.len())
            );
        }
        stdout.flush().unwrap();
        let mut c;
        let mut placement: usize = 0;
        let mut line: usize = 0;
        let mut edit = false;
        let mut clip = Vec::new();
        let mut result: Vec<u8>;
        let mut cursor = 0;
        //let mut start = 0;
        //let mut end = 0;
        //let mut top = 0;
        //let mut bot = 0;
        let mut time = None;
        loop
        {
            c = read_single_char();
            if debug
            {
                time = Some(std::time::Instant::now());
            }
            match c
            {
                '\n' =>
                {
                    if line + 1 == lines.len() && placement == 0
                    {
                        lines.push(Vec::new());
                        println!();
                    }
                    else
                    {
                        lines.insert(line + 1, lines[line][placement..].to_vec());
                        lines.insert(line + 1, lines[line][..placement].to_vec());
                        lines.remove(line);
                        print!(
                            "\x1b[J\n{}\n\x1B[{}A",
                            lines[line + 1..]
                                .iter()
                                .map(|vec| vec.iter().collect::<String>())
                                .collect::<Vec<String>>()
                                .join("\n")
                                .replace('\t', " "),
                            lines.len() - line - 1
                        )
                    }
                    line += 1;
                    if history.pos != 0
                    {
                        history.pos = 0;
                        history.list.clear();
                    }
                    history.list.insert(
                        0,
                        Point {
                            add: true,
                            split: true,
                            pos: (line, placement),
                            char: '\n',
                            line: None,
                        },
                    );
                    placement = 0;
                    cursor = placement;
                }
                '\x08' =>
                {
                    if placement == 0
                    {
                        if line == 0
                        {
                            continue;
                        }
                        line -= 1;
                        placement = lines[line].len();
                        let t = lines.remove(line + 1);
                        lines[line].extend(t);
                        print!(
                            "\x1B[A\x1B[J{}\n\x1B[{}A\x1B[{}C",
                            lines[line..]
                                .iter()
                                .map(|vec| vec.iter().collect::<String>())
                                .collect::<Vec<String>>()
                                .join("\n")
                                .replace('\t', " "),
                            lines.len() - line,
                            placement
                        );
                        if history.pos != 0
                        {
                            history.pos = 0;
                            history.list.clear();
                        }
                        history.list.insert(
                            0,
                            Point {
                                add: false,
                                split: true,
                                pos: (line, placement),
                                char: '\0',
                                line: None,
                            },
                        )
                    }
                    else
                    {
                        placement -= 1;
                        if history.pos != 0
                        {
                            history.pos = 0;
                            history.list.clear();
                        }
                        history.list.insert(
                            0,
                            Point {
                                add: false,
                                split: false,
                                pos: (line, placement),
                                char: lines[line].remove(placement),
                                line: None,
                            },
                        );
                        if placement == lines[line].len()
                        {
                            print!("\x08\x1B[K");
                        }
                        else
                        {
                            print!(
                                "\x1B[K\x1B[G{}\x1B[{}D",
                                lines[line].iter().collect::<String>().replace('\t', " "),
                                lines[line].len() - placement
                            );
                        }
                    }
                    cursor = placement;
                }
                '\x1B' =>
                {
                    //left
                    if placement == 0
                    {
                        if line == 0
                        {
                            continue;
                        }
                        line -= 1;
                        placement = lines[line].len();
                        print!("\x1B[A\x1b[{}C", placement);
                    }
                    else
                    {
                        placement -= 1;
                        print!("\x1B[D",);
                    }
                    cursor = placement;
                }
                '\x1C' =>
                {
                    //right
                    if placement == lines[line].len()
                    {
                        if line + 1 != lines.len()
                        {
                            println!();
                            placement = 0;
                            line += 1;
                        }
                    }
                    else
                    {
                        print!("\x1b[C",);
                        placement += 1;
                    }
                    cursor = placement;
                }
                '\x1D' =>
                {
                    //up
                    if line != 0
                    {
                        line -= 1;
                        print!("\x1B[A");
                        if cursor != 0
                        {
                            if lines[line].len() > cursor
                            {
                                print!("\x1b[G\x1b[{}C", cursor);
                                placement = cursor;
                            }
                            else if placement < cursor || lines[line].len() < placement
                            {
                                print!("\x1b[G\x1b[{}C", lines[line].len());
                                placement = lines[line].len();
                            }
                        }
                    }
                }
                '\x1E' =>
                {
                    //down
                    if line + 1 == lines.len() && !lines[line].is_empty()
                    {
                        lines.push(Vec::new());
                    }
                    if line + 1 != lines.len()
                    {
                        line += 1;
                        print!("\x1B[B");
                        if lines[line].is_empty()
                        {
                            print!("\x1b[G");
                            placement = 0;
                        }
                        else if cursor != 0
                        {
                            if lines[line].len() > cursor
                            {
                                print!("\x1b[G\x1b[{}C", cursor);
                                placement = cursor;
                            }
                            else if placement < cursor || lines[line].len() < placement
                            {
                                print!("\x1b[G\x1b[{}C", lines[line].len());
                                placement = lines[line].len();
                            }
                        }
                    }
                }
                '\x1A' => edit = false,
                _ =>
                {
                    if edit
                    {
                        lines[line].insert(placement, c);
                        print!(
                            "\x1B[K{}{}",
                            lines[line][placement..]
                                .iter()
                                .collect::<String>()
                                .replace('\t', " "),
                            if lines[line].len() - 1 == placement
                            {
                                "".to_string()
                            }
                            else
                            {
                                "\x1B[".to_owned()
                                    + &((lines[line].len() - 1) - placement).to_string()
                                    + "D"
                            }
                        );
                        placement += 1;
                        cursor = placement;
                        if history.pos != 0
                        {
                            history.pos = 0;
                            history.list.clear();
                        }
                        history.list.insert(
                            0,
                            Point {
                                add: true,
                                split: false,
                                pos: (line, placement),
                                char: c,
                                line: None,
                            },
                        )
                    }
                    else if c == 'w'
                    {
                        result = lines
                            .iter()
                            .map(|line| {
                                line.iter()
                                    .collect::<String>()
                                    .trim_end()
                                    .chars()
                                    .map(|c| c as u8)
                                    .collect::<Vec<u8>>()
                            })
                            .flat_map(|line| line.into_iter().chain(std::iter::once(b'\n')))
                            .collect();
                        result.pop();
                        while let Some(last) = result.last()
                        {
                            if *last == b'\n'
                            {
                                result.pop();
                                lines.pop();
                            }
                            else
                            {
                                break;
                            }
                        }
                        result.push(b'\n');
                        File::create(i.clone()).unwrap().write_all(&result).unwrap();
                        if !history.list.is_empty()
                        {
                            if history_file.is_empty()
                            {
                                history_file = history_dir.clone()
                                    + &canonicalize(i.clone())
                                        .unwrap()
                                        .to_str()
                                        .unwrap()
                                        .replace('/', "%");
                            }
                            File::create(history_file.clone())
                                .unwrap()
                                .write_all(&history.to_bytes())
                                .unwrap();
                        }
                    }
                    else if c == 'd'
                    {
                        if !lines.is_empty()
                        {
                            if line + 1 == lines.len()
                            {
                                clip = lines.remove(line);
                                line = line.saturating_sub(1);
                                placement = 0;
                                cursor = 0;
                                print!("\x1b[K\x1b[A");
                            }
                            else
                            {
                                clip = lines.remove(line);
                                placement = 0;
                                cursor = 0;
                                print!(
                                    "\x1b[G\x1b[J{}\n\x1B[{}A",
                                    lines[line..]
                                        .iter()
                                        .map(|vec| vec.iter().collect::<String>())
                                        .collect::<Vec<String>>()
                                        .join("\n")
                                        .replace('\t', " "),
                                    lines.len() - line
                                )
                            }
                            if lines.is_empty()
                            {
                                lines.push(Vec::new());
                            }
                            if history.pos != 0
                            {
                                history.pos = 0;
                                history.list.clear();
                            }
                            history.list.insert(
                                0,
                                Point {
                                    add: false,
                                    split: false,
                                    pos: (line, placement),
                                    char: '\0',
                                    line: Some(clip.clone()),
                                },
                            )
                        }
                    }
                    else if c == 'p'
                    {
                        lines.insert(line, clip.clone());
                        placement = 0;
                        cursor = 0;
                        print!(
                            "\x1b[J{}\n\x1B[{}A",
                            lines[line..]
                                .iter()
                                .map(|vec| vec.iter().collect::<String>())
                                .collect::<Vec<String>>()
                                .join("\n")
                                .replace('\t', " "),
                            lines.len() - line
                        );
                        if history.pos != 0
                        {
                            history.pos = 0;
                            history.list.clear();
                        }
                        history.list.insert(
                            0,
                            Point {
                                add: true,
                                split: false,
                                pos: (line, placement),
                                char: '\0',
                                line: Some(clip.clone()),
                            },
                        )
                    }
                    else if c == '/'
                    {
                        let mut ln = (0, 0);
                        let mut word = Vec::new();
                        loop
                        {
                            c = read_single_char();
                            match c
                            {
                                '\x1A' => break,
                                _ =>
                                {
                                    if c != '\n'
                                    {
                                        ln = (0, 0);
                                        word.push(c);
                                    }
                                    'inner: for (l, i) in lines.iter().enumerate()
                                    {
                                        if (l > ln.0 || ln.0 == 0) && word.len() < i.len()
                                        {
                                            for j in if l == 0 { ln.1 + 1 } else { 0 }
                                                ..=(i.len() - word.len())
                                            {
                                                if i[j..j + word.len()] == word
                                                {
                                                    ln = (l, j);
                                                    print!(
                                                        "\x1B[H{}{}",
                                                        if ln.0 == 0
                                                        {
                                                            "".to_string()
                                                        }
                                                        else
                                                        {
                                                            "\x1B[".to_owned()
                                                                + ln.0.to_string().as_str()
                                                                + "B"
                                                        },
                                                        if ln.1 == 0
                                                        {
                                                            "".to_string()
                                                        }
                                                        else
                                                        {
                                                            "\x1B[".to_owned()
                                                                + ln.1.to_string().as_str()
                                                                + "C"
                                                        },
                                                    );
                                                    stdout.flush().unwrap();
                                                    (line, placement) = ln;
                                                    cursor = placement;
                                                    break 'inner;
                                                }
                                            }
                                            ln = (0, 0);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    else if c == 'q'
                    {
                        if args.len() == n + 1
                        {
                            print!("\x1B[G\x1B[{}B\x1B[?1049l", height);
                        }
                        else
                        {
                            print!("\x1B[H\x1B[J");
                        }
                        stdout.flush().unwrap();
                        continue 'outer;
                    }
                    else if c == 'i'
                    {
                        edit = true;
                    }
                    else if (c == 'z' || c == 'u') && history.list.len() != history.pos
                    {
                        match (
                            history.list[history.pos].add,
                            history.list[history.pos].split,
                            &history.list[history.pos].line,
                        )
                        {
                            (false, false, None) =>
                            {
                                line = history.list[history.pos].pos.0;
                                placement = history.list[history.pos].pos.1;
                                if line == lines.len()
                                {
                                    lines.push(Vec::new());
                                }
                                lines[line].insert(placement, history.list[history.pos].char);
                                placement += 1;
                            }
                            (true, false, None) =>
                            {
                                line = history.list[history.pos].pos.0;
                                placement = history.list[history.pos].pos.1 - 1;
                                lines[line].remove(placement);
                            }
                            (false, true, None) =>
                            {
                                line = history.list[history.pos].pos.0 + 1;
                                placement = 0;
                                let l = lines[line - 1]
                                    .drain(history.list[history.pos].pos.1..)
                                    .collect();
                                lines.insert(line, l);
                            }
                            (true, true, None) =>
                            {
                                line = history.list[history.pos].pos.0 - 1;
                                placement = lines[line].len();
                                let l = lines.remove(line + 1);
                                lines[line].extend(l);
                            }
                            (false, false, Some(l)) =>
                            {
                                line = history.list[history.pos].pos.0;
                                placement = 0;
                                if line == lines.len()
                                {
                                    lines.push(Vec::new());
                                }
                                lines.insert(line, l.clone());
                            }
                            (true, false, Some(_)) =>
                            {
                                line = history.list[history.pos].pos.0;
                                placement = 0;
                                lines.remove(line);
                            }
                            _ => unimplemented!(),
                        }
                        cursor = placement;
                        print!(
                            "\x1B[H\x1B[J{}\x1B[H{}{}",
                            lines
                                .iter()
                                .map(|vec| vec.iter().collect::<String>())
                                .collect::<Vec<String>>()
                                .join("\n")
                                .replace('\t', " "),
                            if line == 0
                            {
                                "".to_string()
                            }
                            else
                            {
                                "\x1B[".to_owned() + &line.to_string() + "B"
                            },
                            if placement == 0
                            {
                                "".to_string()
                            }
                            else
                            {
                                "\x1B[".to_owned() + &placement.to_string() + "C"
                            }
                        );
                        history.pos += 1;
                    }
                    else if (c == 'x' || c == 'r') && history.pos > 0
                    {
                        history.pos -= 1;
                        match (
                            history.list[history.pos].add,
                            history.list[history.pos].split,
                            &history.list[history.pos].line,
                        )
                        {
                            (false, false, None) =>
                            {
                                line = history.list[history.pos].pos.0;
                                placement = history.list[history.pos].pos.1;
                                lines[line].remove(placement);
                            }
                            (true, false, None) =>
                            {
                                line = history.list[history.pos].pos.0;
                                placement = history.list[history.pos].pos.1 - 1;
                                if line == lines.len()
                                {
                                    lines.push(Vec::new());
                                }
                                lines[line].insert(placement, history.list[history.pos].char);
                                placement += 1;
                            }
                            (false, true, None) =>
                            {
                                line = history.list[history.pos].pos.0;
                                placement = lines[line].len();
                                let l = lines.remove(line + 1);
                                lines[line].extend(l);
                            }
                            (true, true, None) =>
                            {
                                line = history.list[history.pos].pos.0 + 1;
                                placement = 0;
                                let l = lines[line]
                                    .drain(history.list[history.pos].pos.1..)
                                    .collect();
                                lines.insert(line, l);
                            }
                            (false, false, Some(_)) =>
                            {
                                line = history.list[history.pos].pos.0;
                                placement = 0;
                                lines.remove(line);
                            }
                            (true, false, Some(l)) =>
                            {
                                line = history.list[history.pos].pos.0;
                                placement = 0;
                                if line == lines.len()
                                {
                                    lines.push(Vec::new());
                                }
                                lines.insert(line, l.clone());
                            }
                            _ => unimplemented!(),
                        }
                        cursor = placement;
                        print!(
                            "\x1B[H\x1B[J{}\x1B[H{}{}",
                            lines
                                .iter()
                                .map(|vec| vec.iter().collect::<String>())
                                .collect::<Vec<String>>()
                                .join("\n")
                                .replace('\t', " "),
                            if line == 0
                            {
                                "".to_string()
                            }
                            else
                            {
                                "\x1B[".to_owned() + &line.to_string() + "B"
                            },
                            if placement == 0
                            {
                                "".to_string()
                            }
                            else
                            {
                                "\x1B[".to_owned() + &placement.to_string() + "C"
                            }
                        );
                    }
                }
            }
            if debug
            {
                print!(
                    "\x1B[{}B\x1B[G\x1B[K{}\x1B[H{}{}",
                    height,
                    time.unwrap().elapsed().as_nanos(),
                    if line == 0
                    {
                        "".to_string()
                    }
                    else
                    {
                        "\x1B[".to_owned() + &line.to_string() + "B"
                    },
                    if placement == 0
                    {
                        "".to_string()
                    }
                    else
                    {
                        "\x1B[".to_owned() + &placement.to_string() + "C"
                    }
                );
            }
            stdout.flush().unwrap();
        }
    }
}
fn read_single_char() -> char
{
    let term = Term::stdout();
    match term.read_key().unwrap()
    {
        Key::Char(c) => c,
        Key::Enter => '\n',
        Key::Backspace => '\x08',
        Key::ArrowLeft => '\x1B',
        Key::ArrowRight => '\x1C',
        Key::ArrowUp => '\x1D',
        Key::ArrowDown => '\x1E',
        Key::Escape => '\x1A',
        Key::Tab => '\t',
        _ => '\0',
    }
}
fn get_dimensions() -> (usize, usize)
{
    unsafe {
        let mut size: winsize = mem::zeroed();
        ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut size);
        (size.ws_row as usize, size.ws_col as usize)
    }
}
struct History
{
    pos: usize,
    list: Vec<Point>,
}
struct Point
{
    add: bool,
    split: bool,
    pos: (usize, usize),
    char: char,
    line: Option<Vec<char>>,
}
impl History
{
    fn to_bytes(&self) -> Vec<u8>
    {
        let mut bytes = Vec::new();
        bytes.extend(&self.pos.to_le_bytes());
        bytes.extend(&self.list.len().to_le_bytes());
        for point in &self.list
        {
            let point_bytes = point.to_bytes();
            bytes.extend(&point_bytes.len().to_le_bytes());
            bytes.extend(point_bytes);
        }
        bytes
    }
    fn from_bytes(bytes: &[u8]) -> History
    {
        let mut cursor = 0;
        let pos = usize::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]);
        cursor += 8;
        let list_len = usize::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]);
        cursor += 8;
        let mut list = Vec::with_capacity(list_len);
        for _ in 0..list_len
        {
            let point_size = usize::from_le_bytes([
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
            ]);
            cursor += 8;
            let point_bytes = &bytes[cursor..cursor + point_size];
            list.push(Point::from_bytes(point_bytes));
            cursor += point_size;
        }
        History { pos, list }
    }
}
impl Point
{
    fn to_bytes(&self) -> Vec<u8>
    {
        let mut bytes = Vec::new();
        bytes.extend(&[self.add as u8, self.split as u8]);
        bytes.extend(&self.pos.0.to_le_bytes());
        bytes.extend(&self.pos.1.to_le_bytes());
        bytes.push(self.char as u8);
        match &self.line
        {
            Some(line) =>
            {
                bytes.push(1);
                bytes.extend(&line.len().to_le_bytes());
                bytes.extend(line.iter().map(|&c| c as u8));
            }
            None =>
            {
                bytes.push(0);
            }
        }
        bytes
    }
    fn from_bytes(bytes: &[u8]) -> Point
    {
        let mut cursor = 0;
        let add = bytes[cursor] != 0;
        cursor += 1;
        let split = bytes[cursor] != 0;
        cursor += 1;
        let pos_0 = usize::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]);
        cursor += 8;
        let pos_1 = usize::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]);
        cursor += 8;
        let char = bytes[cursor] as char;
        cursor += 1;
        let line = if bytes[cursor] == 1
        {
            cursor += 1;
            let len = usize::from_le_bytes([
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
            ]);
            cursor += 8;
            let mut vec = Vec::with_capacity(len);
            for _ in 0..len
            {
                vec.push(bytes[cursor] as char);
                cursor += 1;
            }
            Some(vec)
        }
        else
        {
            None
        };
        Point {
            add,
            split,
            pos: (pos_0, pos_1),
            char,
            line,
        }
    }
}