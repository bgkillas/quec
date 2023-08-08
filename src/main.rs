mod history;
use console::{Key, Term};
use history::{History, Point};
use std::{
    cmp::Ordering,
    env::args,
    fs::{canonicalize, create_dir, File},
    io::{stdout, BufRead, BufReader, Read, Write},
};
#[cfg(unix)]
use {
    libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ},
    std::mem,
};
#[cfg(not(unix))]
use {std::env::var, term_size::dimensions};
//TODO package for windows
//TODO word wrapping
fn main()
{
    let term = Term::stdout();
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
                help();
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
    print!("\x1B[?1049h\x1B[H\x1B[J");
    stdout.flush().unwrap();
    #[cfg(unix)]
    let history_dir = env!("HOME").to_owned() + "/.quec/";
    #[cfg(not(unix))]
    let history_dir = &format!(
        "C:\\Users\\{}\\AppData\\Roaming\\quec\\",
        var("USERNAME").unwrap()
    );
    let _ = create_dir(history_dir.clone());
    let (mut height, mut width) = get_dimensions();
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
            #[cfg(unix)]
            {
                history_file = history_dir.clone()
                    + &canonicalize(i.clone())
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('/', "%");
            }
            #[cfg(not(unix))]
            {
                history_file = canonicalize(i.clone())
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "%");
                history_file =
                    history_dir.clone() + &history_file[history_file.find(':').unwrap()..];
            }
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
        let mut search = false;
        let mut ln: Option<(usize, usize)> = None;
        let mut word: Vec<char> = Vec::new();
        let mut clip = Vec::new();
        let mut result: Vec<u8>;
        let mut cursor = 0;
        let mut top = 0;
        let mut time = None;
        loop
        {
            if (height, width) != get_dimensions()
            {
                (height, width) = get_dimensions();
                top = match top.cmp(&line)
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
                };
                clear(&lines, top, height);
            }
            if history.list.len() >= 1000
            {
                history.list.drain(1000..);
            }
            if history.pos > history.list.len()
            {
                history.list.clear();
            }
            c = read_single_char(&term);
            if debug
            {
                time = Some(std::time::Instant::now());
            }
            match c
            {
                '\n' if !search =>
                {
                    if edit
                    {
                        line += 1;
                        if line == lines.len() && placement == 0
                        {
                            lines.push(Vec::new());
                            placement = 0;
                            cursor = placement;
                        }
                        else
                        {
                            lines.insert(line, lines[line - 1][placement..].to_vec());
                            lines.insert(line, lines[line - 1][..placement].to_vec());
                            lines.remove(line - 1);
                            placement = 0;
                            cursor = placement;
                            clear(&lines, top, height);
                        }
                        if history.pos != 0
                        {
                            history.list.drain(..history.pos);
                            history.pos = 0;
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
                        if line == height + top
                        {
                            top += 1;
                            clear(&lines, top, height);
                        }
                    }
                }
                '\x08' =>
                {
                    if edit
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
                            clear(&lines, top, height);
                            if history.pos != 0
                            {
                                history.list.drain(..history.pos);
                                history.pos = 0;
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
                                history.list.drain(..history.pos);
                                history.pos = 0;
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
                                    "\x1B[K\x1B[G{}",
                                    lines[line].iter().collect::<String>().replace('\t', " ")
                                );
                            }
                        }
                        cursor = placement;
                    }
                    else if search && !word.is_empty()
                    {
                        word.pop();
                        print!(
                            "\x1B[G\x1B[{}B\x1B[{}C\x1B[K{}",
                            height,
                            width - 30,
                            word.iter().collect::<String>()
                        );
                    }
                }
                '\x01' =>
                {
                    //home
                    placement = 0;
                    line = 0;
                    if lines.len() > height
                    {
                        top = 0;
                        clear(&lines, top, height);
                    }
                    if search
                    {
                        ln = Some((line, placement));
                    }
                }
                '\x02' =>
                {
                    //end
                    line = lines.len() - 1;
                    placement = lines[line].len();
                    if lines.len() > height
                    {
                        top = lines.len() - height;
                        clear(&lines, top, height);
                    }
                    if search
                    {
                        ln = Some((line, placement));
                    }
                }
                '\x03' =>
                {
                    //page up
                    if line < height
                    {
                        top = 0;
                        placement = 0;
                        line = 0;
                    }
                    else
                    {
                        placement = 0;
                        line -= height;
                        top -= height;
                        clear(&lines, top, height);
                    }
                    if search
                    {
                        ln = Some((line, placement));
                    }
                }
                '\x04' =>
                {
                    //page down
                    if line + height >= lines.len()
                    {
                        top = lines.len().saturating_sub(height);
                        line = lines.len() - 1;
                        placement = lines[line].len();
                        clear(&lines, top, height);
                    }
                    else
                    {
                        placement = 0;
                        line += height;
                        top += height;
                        clear(&lines, top, height);
                    }
                    if search
                    {
                        ln = Some((line, placement));
                    }
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
                        if line + 1 == top
                        {
                            top -= 1;
                            clear(&lines, top, height);
                        }
                    }
                    else
                    {
                        placement -= 1;
                    }
                    cursor = placement;
                    if search
                    {
                        ln = Some((line, placement));
                    }
                }
                '\x1C' =>
                {
                    //right
                    if placement == lines[line].len()
                    {
                        if line + 1 != lines.len()
                        {
                            placement = 0;
                            line += 1;
                            if line == height + top
                            {
                                top += 1;
                                clear(&lines, top, height);
                            }
                            else
                            {
                                println!()
                            }
                        }
                    }
                    else
                    {
                        placement += 1;
                    }
                    cursor = placement;
                    if search
                    {
                        ln = Some((line, placement));
                    }
                }
                '\x1D' =>
                {
                    //up
                    if line != 0
                    {
                        line -= 1;
                        if cursor != 0
                        {
                            if lines[line].len() > cursor
                            {
                                placement = cursor;
                            }
                            else if placement < cursor || lines[line].len() < placement
                            {
                                if lines[line].is_empty()
                                {
                                    placement = 0;
                                }
                                else
                                {
                                    placement = lines[line].len();
                                }
                            }
                        }
                    }
                    if line + 1 == top
                    {
                        top -= 1;
                        clear(&lines, top, height);
                    }
                    if search
                    {
                        ln = Some((line, placement));
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
                        if lines[line].is_empty()
                        {
                            placement = 0;
                        }
                        else if cursor != 0
                        {
                            if lines[line].len() > cursor
                            {
                                placement = cursor;
                            }
                            else if placement < cursor || lines[line].len() < placement
                            {
                                placement = lines[line].len();
                            }
                        }
                    }
                    if line == height + top
                    {
                        top += 1;
                        clear(&lines, top, height);
                    }
                    if search
                    {
                        ln = Some((line, placement));
                    }
                }
                '\x1A' =>
                {
                    edit = false;
                    search = false;
                    clear(&lines, top, height);
                }
                _ if c.is_ascii_graphic() || c == ' ' || c == '\t' || c == '\n' =>
                {
                    if edit
                    {
                        lines[line].insert(placement, c);
                        print!(
                            "\x1B[K{}",
                            lines[line][placement..]
                                .iter()
                                .collect::<String>()
                                .replace('\t', " ")
                        );
                        placement += 1;
                        cursor = placement;
                        if history.pos != 0
                        {
                            history.list.drain(..history.pos);
                            history.pos = 0;
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
                    else if search
                    {
                        if c != '\n'
                        {
                            ln = None;
                            word.push(c);
                            print!(
                                "\x1B[G\x1B[{}B\x1B[{}C\x1B[K{}",
                                height,
                                width - 30,
                                word.iter().collect::<String>()
                            );
                        }
                        'inner: for (l, i) in lines.iter().enumerate()
                        {
                            if (ln.is_some_and(|x| l >= x.0) || ln.is_none())
                                && word.len() <= i.len()
                            {
                                for j in if let Some(n) = ln { n.1 + 1 } else { 0 }
                                    ..=(i.len() - word.len())
                                {
                                    if i[j..j + word.len()] == word
                                    {
                                        ln = Some((l, j));
                                        (line, placement) = ln.unwrap();
                                        top = match top.cmp(&line)
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
                                        };
                                        cursor = placement;
                                        clear(&lines, top, height);
                                        print!(
                                            "\x1B[G\x1B[{}B\x1B[{}C\x1B[K{}",
                                            height,
                                            width - 30,
                                            word.iter().collect::<String>()
                                        );
                                        break 'inner;
                                    }
                                }
                                ln = None;
                            }
                        }
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
                        loop
                        {
                            if !history.list.is_empty()
                                && (
                                    history.list[0].add,
                                    history.list[0].split,
                                    history.list[0].line.clone(),
                                ) == (true, true, None)
                                && history.list[0].pos.0 >= lines.len()
                            {
                                history.list.remove(0);
                            }
                            else
                            {
                                break;
                            }
                        }
                        if !history.list.is_empty()
                        {
                            if history_file.is_empty()
                            {
                                #[cfg(unix)]
                                {
                                    history_file = history_dir.clone()
                                        + &canonicalize(i.clone())
                                            .unwrap()
                                            .to_str()
                                            .unwrap()
                                            .replace('/', "%");
                                }
                                #[cfg(not(unix))]
                                {
                                    history_file = canonicalize(i.clone())
                                        .unwrap()
                                        .to_str()
                                        .unwrap()
                                        .replace('\\', "%");
                                    history_file = history_dir.clone()
                                        + &history_file[history_file.find(':').unwrap()..];
                                }
                            }
                            println!("{}", history_file);
                            File::create(history_file.clone())
                                .unwrap()
                                .write_all(&history.to_bytes())
                                .unwrap();
                        }
                    }
                    else if c == 'y'
                    {
                        if !lines.is_empty()
                        {
                            clip = lines[line].clone();
                        }
                    }
                    else if c == 'd'
                    {
                        if !lines.is_empty()
                        {
                            if line + 1 == lines.len()
                            {
                                clip = lines.pop().unwrap();
                                lines.push(Vec::new());
                                placement = 0;
                                cursor = 0;
                                print!("\x1b[G\x1b[K");
                            }
                            else
                            {
                                clip = lines.remove(line);
                                placement = 0;
                                cursor = 0;
                                clear(&lines, top, height);
                            }
                            if history.pos != 0
                            {
                                history.list.drain(..history.pos);
                                history.pos = 0;
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
                        clear(&lines, top, height);
                        if history.pos != 0
                        {
                            history.list.drain(..history.pos);
                            history.pos = 0;
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
                        search = true;
                        ln = None;
                        word = Vec::new();
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
                        top = match top.cmp(&line)
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
                        };
                        clear(&lines, top, height);
                        history.pos += 1;
                    }
                    else if c == 'x' && history.pos > 0
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
                                line = history.list[history.pos].pos.0;
                                placement = 0;
                                if line == lines.len()
                                {
                                    lines.push(Vec::new())
                                }
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
                        top = match top.cmp(&line)
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
                        };
                        clear(&lines, top, height);
                    }
                }
                _ =>
                {}
            }
            if debug
            {
                print!(
                    "\x1B[G\x1B[{}B\x1B[{}C\x1B[K{}",
                    height,
                    width - 30,
                    time.unwrap().elapsed().as_nanos()
                );
            }
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
                if placement == 0
                {
                    String::new()
                }
                else
                {
                    "\x1B[".to_owned() + &placement.to_string() + "C"
                }
            );
            stdout.flush().unwrap();
        }
    }
}
fn read_single_char(term: &Term) -> char
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
fn get_dimensions() -> (usize, usize)
{
    unsafe {
        let mut size: winsize = mem::zeroed();
        ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut size);
        (size.ws_row as usize, size.ws_col as usize)
    }
}
#[cfg(not(unix))]
fn get_dimensions() -> (usize, usize)
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
fn help()
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
fn clear(lines: &[Vec<char>], top: usize, height: usize)
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