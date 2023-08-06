use console::{Key, Term};
use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};
use std::{
    env::args,
    fs::File,
    io::{stdout, BufRead, BufReader, Write},
    mem,
};
fn main()
{
    let mut args = args().collect::<Vec<String>>();
    args.remove(0);
    if args.is_empty()
    {
        return;
    }
    let mut stdout = stdout();
    print!("\x1B[K\x1B[J");
    stdout.flush().unwrap();
    let mut lines = if File::open(&args[0]).is_err()
    {
        vec![]
    }
    else
    {
        BufReader::new(File::open(&args[0]).unwrap())
            .lines()
            .map(|l| {
                l.unwrap()
                    .chars()
                    .filter(|c| c.is_ascii_graphic() || c == &' ' || c == &'\t' || c == &'\n')
                    .collect::<Vec<char>>()
            })
            .collect::<Vec<Vec<char>>>()
    };
    //TODO word wrapping and support files longer then screen
    let (height, _width) = get_dimensions();
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
    loop
    {
        c = read_single_char();
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
                }
                else
                {
                    placement -= 1;
                    lines[line].remove(placement);
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
                    if placement != 0
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
                if line + 1 != lines.len()
                {
                    line += 1;
                    print!("\x1B[B");
                    if placement == 0
                    {
                    }
                    else if lines[line].len() > cursor
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
                    File::create(&args[0]).unwrap().write_all(&result).unwrap();
                }
                else if c == 'd'
                {
                    if line != lines.len()
                    {
                        clip = lines.remove(line);
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
                        lines.push(vec![]);
                    }
                }
                else if c == 'p'
                {
                    lines.insert(line, clip.clone());
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
                    print!("\x1B[H\x1B[J");
                    stdout.flush().unwrap();
                    return;
                }
                else if c == 'i'
                {
                    edit = true;
                }
            }
        }
        stdout.flush().unwrap();
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