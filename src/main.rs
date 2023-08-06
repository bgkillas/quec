use console::{Key, Term};
use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};
use std::{
    cmp::Ordering,
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
        std::process::exit(1);
    }
    if File::open(&args[0]).is_err()
    {
        File::create(&args[0]).unwrap();
    }
    let mut stdout = stdout();
    print!("\x1B[K\x1B[J");
    stdout.flush().unwrap();
    let mut lines = BufReader::new(File::open(&args[0]).unwrap())
        .lines()
        .map(|l| l.unwrap().chars().collect::<Vec<char>>())
        .collect::<Vec<Vec<char>>>();
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
            }
            '\x1C' =>
            {
                //right
                if placement == lines[line].len()
                {
                    println!();
                    placement = 0;
                    line += 1;
                }
                else
                {
                    print!("\x1b[C",);
                    placement += 1;
                }
            }
            '\x1D' =>
            {
                //up
                if line != 0
                {
                    line -= 1;
                    print!("\x1B[A");
                }
                if lines[line].len() < placement
                {
                    let len = lines[line].len();
                    lines[line].extend(vec![' '; placement - len])
                }
            }
            '\x1E' =>
            {
                //down
                if line + 1 == lines.len()
                {
                    lines.push(Vec::new());
                }
                line += 1;
                print!("\x1B[B");
                if lines[line].len() < placement
                {
                    let len = lines[line].len();
                    lines[line].extend(vec![' '; placement - len])
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
                        lines[line][placement..].iter().collect::<String>(),
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
                    clip = lines.remove(line);
                    print!(
                        "\x1b[J{}\n\x1B[{}A",
                        lines[line..]
                            .iter()
                            .map(|vec| vec.iter().collect::<String>())
                            .collect::<Vec<String>>()
                            .join("\n")
                            .replace('\t', " "),
                        lines.len() - line
                    )
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
                            '\n' =>
                            {
                                'inner: for (l, i) in lines[ln.0..].iter().enumerate()
                                {
                                    if word.len() < i.len()
                                    {
                                        for j in if l == 0 { ln.1 + 1 } else { 0 }
                                            ..=(i.len() - word.len())
                                        {
                                            if i[j..j + word.len()] == word
                                            {
                                                ln = (l + ln.0, j);
                                                print!(
                                                    "\x1b[H{}{}",
                                                    match line.cmp(&ln.0)
                                                    {
                                                        Ordering::Less =>
                                                        {
                                                            "\x1B[".to_owned()
                                                                + &(ln.0 - line).to_string()
                                                                + "B"
                                                        }
                                                        Ordering::Greater =>
                                                        {
                                                            "\x1B[".to_owned()
                                                                + &(line - ln.0).to_string()
                                                                + "A"
                                                        }
                                                        Ordering::Equal => "".to_string(),
                                                    },
                                                    match placement.cmp(&j)
                                                    {
                                                        Ordering::Less =>
                                                        {
                                                            "\x1B[".to_owned()
                                                                + &(j - placement).to_string()
                                                                + "C"
                                                        }
                                                        Ordering::Greater =>
                                                        {
                                                            "\x1B[".to_owned()
                                                                + &(placement - j).to_string()
                                                                + "D"
                                                        }
                                                        Ordering::Equal => "".to_string(),
                                                    }
                                                );
                                                stdout.flush().unwrap();
                                                break 'inner;
                                            }
                                        }
                                    }
                                }
                            }
                            '\x1A' => break,
                            _ =>
                            {
                                ln = (0, 0);
                                word.push(c);
                                'inner: for (l, i) in lines.iter().enumerate()
                                {
                                    if word.len() < i.len()
                                    {
                                        for j in 0..=(i.len() - word.len())
                                        {
                                            if i[j..j + word.len()] == word
                                            {
                                                ln = (l, j);
                                                print!(
                                                    "\x1b[H{}{}",
                                                    match line.cmp(&l)
                                                    {
                                                        Ordering::Less =>
                                                        {
                                                            "\x1B[".to_owned()
                                                                + &(l - line).to_string()
                                                                + "B"
                                                        }
                                                        Ordering::Greater =>
                                                        {
                                                            "\x1B[".to_owned()
                                                                + &(line - l).to_string()
                                                                + "A"
                                                        }
                                                        Ordering::Equal => "".to_string(),
                                                    },
                                                    match placement.cmp(&j)
                                                    {
                                                        Ordering::Less =>
                                                        {
                                                            "\x1B[".to_owned()
                                                                + &(j - placement).to_string()
                                                                + "C"
                                                        }
                                                        Ordering::Greater =>
                                                        {
                                                            "\x1B[".to_owned()
                                                                + &(placement - j).to_string()
                                                                + "D"
                                                        }
                                                        Ordering::Equal => "".to_string(),
                                                    }
                                                );
                                                stdout.flush().unwrap();
                                                break 'inner;
                                            }
                                        }
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