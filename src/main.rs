mod file;
mod history;
mod misc;
use crate::{
    file::{get_word, open_file, save_file, Files},
    history::{History, Point},
    misc::{
        clear, clear_line, exit, fix_history, fix_top, get_dimensions, help, print_line_number,
        read_single_char,
    },
    Mode::{Default, Digraph, Edit, Insert, Search},
};
use crossterm::terminal;
use std::{
    cmp::min,
    env::{args, var},
    fs::{canonicalize, create_dir},
    io::{stdout, Write},
};
fn main()
{
    let args = &args().collect::<Vec<String>>()[1..];
    if !args.is_empty()
    {
        match args[0].as_str()
        {
            "--help" | "-h" =>
            {
                help();
                std::process::exit(0);
            }
            "--version" | "-v" =>
            {
                println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ =>
            {}
        }
    }
    let mut stdout = stdout();
    print!("\x1b[?1049h\x1b[H\x1b[J");
    #[cfg(unix)]
    let history_dir = var("HOME").unwrap() + "/.quec/";
    #[cfg(not(unix))]
    let history_dir = &format!(
        "C:\\Users\\{}\\AppData\\Roaming\\quec\\",
        var("USERNAME").unwrap()
    );
    let _ = create_dir(history_dir.clone());
    let (mut height, mut width) = get_dimensions();
    let mut files: Vec<Files>;
    if args.is_empty()
    {
        files = vec![Files {
            lines: vec![Vec::new()],
            history: History {
                pos: 0,
                list: Vec::new(),
            },
            save_file_path: String::new(),
            history_file: String::new(),
            placement: 0,
            line: 0,
            start: 0,
            top: 0,
            cursor: 0,
        }]
    }
    else
    {
        files = Vec::with_capacity(args.len());
        for arg in args
        {
            files.push(open_file(arg, &history_dir));
        }
    }
    let mut n = 0;
    let mut clip = Vec::new();
    terminal::enable_raw_mode().unwrap();
    'outer: loop
    {
        let mut top = files[n].top;
        let mut start = files[n].start;
        let mut line = files[n].line;
        let mut placement = files[n].placement;
        clear(&files[n].lines, top, height, start, width);
        print_line_number(
            height,
            line,
            placement,
            top,
            start,
            files[n].save_file_path.clone(),
        );
        stdout.flush().unwrap();
        let mut saved = true;
        let mut err = String::new();
        let mut mode: Mode = Default;
        let mut ln: (usize, usize) = (0, 0);
        let mut orig: (usize, usize) = (0, 0);
        let mut word: Vec<char> = Vec::new();
        loop
        {
            if (height, width) != get_dimensions()
            {
                (height, width) = get_dimensions();
                top = fix_top(top, line, height);
                start = fix_top(start, placement, width);
                clear(&files[n].lines, top, height, start, width);
            }
            if files[n].history.list.len() >= 1000
            {
                files[n].history.list.drain(1000..);
            }
            if files[n].history.pos > files[n].history.list.len()
            {
                files[n].history.list.clear();
            }
            let c = read_single_char();
            match c
            {
                '\n' if mode != Search =>
                {
                    //enter
                    if mode == Edit || mode == Digraph
                    {
                        saved = false;
                        line += 1;
                        let mut ln: Vec<char> = files[n].lines[line - 1][..placement]
                            .iter()
                            .take_while(|&&c| c == ' ' || c == '\t')
                            .cloned()
                            .collect();
                        let count = ln.len();
                        ln.extend::<Vec<char>>(
                            files[n].lines[line - 1].drain(placement..).collect(),
                        );
                        files[n].lines.insert(line, ln);
                        placement = count;
                        files[n].cursor = placement;
                        start = 0;
                        if line == height + top
                        {
                            top += 1;
                        }
                        clear(&files[n].lines, top, height, start, width);
                        fix_history(&mut files[n].history);
                        files[n].history.list.insert(
                            0,
                            Point {
                                add: true,
                                split: true,
                                pos: (line, placement),
                                char: '\n',
                                line: None,
                            },
                        );
                    }
                    else
                    {
                        if line + 1 != files[n].lines.len()
                        {
                            line += 1;
                        }
                        placement = files[n].lines[line]
                            .iter()
                            .take_while(|&&c| c == ' ' || c == '\t')
                            .count();
                        let s = start;
                        start = fix_top(start, placement, width);
                        if line == height + top
                        {
                            top += 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                        else if s != start
                        {
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                }
                '\x08' =>
                {
                    //backspace
                    if mode == Edit || mode == Digraph
                    {
                        saved = false;
                        if placement == 0
                        {
                            if line == 0
                            {
                                continue;
                            }
                            line -= 1;
                            placement = files[n].lines[line].len();
                            let t = files[n].lines.remove(line + 1);
                            files[n].lines[line].extend(t);
                            start = fix_top(start, placement, width);
                            clear(&files[n].lines, top, height, start, width);
                            print!("\x1b[E\x1b[G\x1b[K");
                            fix_history(&mut files[n].history);
                            files[n].history.list.insert(
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
                            fix_history(&mut files[n].history);
                            let ln = files[n].lines[line].remove(placement);
                            files[n].history.list.insert(
                                0,
                                Point {
                                    add: false,
                                    split: false,
                                    pos: (line, placement),
                                    char: ln,
                                    line: None,
                                },
                            );
                            if placement == files[n].lines[line].len()
                            {
                                if placement + 1 == start
                                {
                                    start -= 1;
                                    clear(&files[n].lines, top, height, start, width);
                                }
                                else
                                {
                                    print!("\x1b[D\x1b[K");
                                }
                            }
                            else if placement + 1 == start
                            {
                                start -= 1;
                                clear(&files[n].lines, top, height, start, width);
                            }
                            else
                            {
                                print!("\x1b[D");
                                clear_line(&files[n].lines, line, placement, width, start)
                            }
                        }
                        files[n].cursor = placement;
                    }
                    else if mode == Search && !word.is_empty()
                    {
                        word.pop();
                    }
                }
                '\x15' if (mode == Edit || mode == Digraph) && placement != 0 =>
                {
                    //ctrl+back
                    saved = false;
                    let initial = placement;
                    let mut did = false;
                    let mut on_white = files[n].lines[line][placement - 1].is_whitespace();
                    for (i, c) in files[n].lines[line][..placement - 1]
                        .iter()
                        .rev()
                        .enumerate()
                    {
                        if c.is_whitespace()
                        {
                            if !on_white
                            {
                                placement -= i;
                                placement -= 1;
                                did = true;
                                files[n].cursor = placement;
                                break;
                            }
                        }
                        else
                        {
                            on_white = false;
                        }
                    }
                    if !did
                    {
                        placement = 0;
                    }
                    fix_history(&mut files[n].history);
                    let ln = files[n].lines[line].drain(placement..initial).collect();
                    files[n].history.list.insert(
                        0,
                        Point {
                            add: false,
                            split: true,
                            pos: (line, placement),
                            char: '\0',
                            line: Some(ln),
                        },
                    );
                    if start > placement
                    {
                        start = placement;
                        clear(&files[n].lines, top, height, start, width);
                    }
                    else
                    {
                        print!("\x1b[{}D", initial - placement);
                        clear_line(&files[n].lines, line, placement, width, start)
                    }
                }
                '\x01' =>
                {
                    //home
                    placement = 0;
                    line = 0;
                    if top != 0 || start != 0
                    {
                        top = 0;
                        start = 0;
                        clear(&files[n].lines, top, height, start, width);
                    }
                    files[n].cursor = placement;
                    if mode == Search
                    {
                        ln = (line, placement);
                    }
                }
                '\x02' =>
                {
                    //end
                    line = files[n].lines.len() - 1;
                    placement = files[n].lines[line].len();
                    if files[n].lines.len() > height || placement > width
                    {
                        top = files[n].lines.len().saturating_sub(height);
                        start = placement.saturating_sub(width);
                        clear(&files[n].lines, top, height, start, width);
                    }
                    files[n].cursor = placement;
                    if mode == Search
                    {
                        ln = (line, placement);
                    }
                }
                '\x03' =>
                {
                    //page up
                    placement = 0;
                    if line < height
                    {
                        top = 0;
                        line = 0;
                        let s = start;
                        start = fix_top(start, placement, width);
                        if s != 0
                        {
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    else
                    {
                        line -= height;
                        if height < top
                        {
                            top -= height;
                        }
                        else
                        {
                            top = 0;
                        }
                        start = fix_top(start, placement, width);
                        clear(&files[n].lines, top, height, start, width);
                    }
                    files[n].cursor = placement;
                    if mode == Search
                    {
                        ln = (line, placement);
                    }
                }
                '\x04' =>
                {
                    //page down
                    if line + height >= files[n].lines.len()
                    {
                        top = files[n].lines.len().saturating_sub(height);
                        line = files[n].lines.len() - 1;
                        placement = files[n].lines[line].len();
                        start = fix_top(start, placement, width);
                        clear(&files[n].lines, top, height, start, width);
                    }
                    else
                    {
                        placement = 0;
                        line += height;
                        top += height;
                        if top + height > files[n].lines.len()
                        {
                            top = files[n].lines.len() - height;
                        }
                        start = fix_top(start, placement, width);
                        clear(&files[n].lines, top, height, start, width);
                    }
                    files[n].cursor = placement;
                    if mode == Search
                    {
                        ln = (line, placement);
                    }
                }
                '\x12' if placement != 0 =>
                {
                    //ctrl+left
                    let mut did = false;
                    let mut on_white = files[n].lines[line][placement - 1].is_whitespace();
                    for (i, c) in files[n].lines[line][..placement - 1]
                        .iter()
                        .rev()
                        .enumerate()
                    {
                        if c.is_whitespace()
                        {
                            if !on_white
                            {
                                placement -= i;
                                placement -= 1;
                                did = true;
                                break;
                            }
                        }
                        else
                        {
                            on_white = false;
                        }
                    }
                    if !did
                    {
                        placement = 0;
                    }
                    files[n].cursor = placement;
                    if start > placement
                    {
                        start = placement;
                        clear(&files[n].lines, top, height, start, width);
                    }
                }
                '\x13' if files[n].lines[line].len() != placement =>
                {
                    //ctrl+right
                    let mut did = false;
                    let mut on_white = files[n].lines[line][placement].is_whitespace();
                    for (i, c) in files[n].lines[line][placement..].iter().enumerate()
                    {
                        if !c.is_whitespace()
                        {
                            if on_white
                            {
                                placement += i;
                                did = true;
                                break;
                            }
                        }
                        else
                        {
                            on_white = true
                        }
                    }
                    if !did
                    {
                        placement = files[n].lines[line].len();
                    }
                    files[n].cursor = placement;
                    if start + width < placement
                    {
                        start += placement - (start + width) + 1;
                        clear(&files[n].lines, top, height, start, width);
                    }
                }
                '\x1B' | 'h' if c != 'h' || mode == Default =>
                {
                    //left
                    if placement == 0
                    {
                        if line == 0
                        {
                            continue;
                        }
                        line -= 1;
                        placement = files[n].lines[line].len();
                        let s = start;
                        start = fix_top(start, placement, width);
                        if line + 1 == top
                        {
                            top -= 1;
                            if placement + 1 == start
                            {
                                start -= 1;
                            }
                            clear(&files[n].lines, top, height, start, width);
                        }
                        else if start != s
                        {
                            if placement + 1 == start
                            {
                                start -= 1;
                            }
                            clear(&files[n].lines, top, height, start, width);
                        }
                        else if placement + 1 == start
                        {
                            start -= 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    else
                    {
                        placement -= 1;
                        if placement + 1 == start
                        {
                            start -= 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    files[n].cursor = placement;
                    if mode == Search
                    {
                        ln = (line, placement);
                    }
                }
                '\x1C' | 'l' if c != 'l' || mode == Default =>
                {
                    //right
                    if placement == files[n].lines[line].len()
                    {
                        if line + 1 != files[n].lines.len()
                        {
                            placement = 0;
                            line += 1;
                            let s = start;
                            start = fix_top(start, placement, width);
                            if line == height + top
                            {
                                top += 1;
                                if placement == width + start
                                {
                                    start += 1;
                                }
                                clear(&files[n].lines, top, height, start, width);
                            }
                            else if start != s
                            {
                                if placement == width + start
                                {
                                    start += 1;
                                }
                                clear(&files[n].lines, top, height, start, width);
                            }
                            else if placement == width + start
                            {
                                start += 1;
                                clear(&files[n].lines, top, height, start, width);
                            }
                        }
                    }
                    else
                    {
                        placement += 1;
                        if placement == width + start
                        {
                            start += 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    files[n].cursor = placement;
                    if mode == Search
                    {
                        ln = (line, placement);
                    }
                }
                '\x1D' | 'k' if c != 'k' || mode == Default =>
                {
                    //up
                    if line == 0
                    {
                        placement = 0;
                        files[n].cursor = 0;
                        if start != 0
                        {
                            start = 0;
                            if line + 1 == top
                            {
                                top -= 1;
                            }
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    else
                    {
                        line -= 1;
                        if files[n].cursor != 0
                        {
                            if files[n].lines[line].len() > files[n].cursor
                            {
                                placement = files[n].cursor;
                            }
                            else if placement < files[n].cursor
                                || files[n].lines[line].len() < placement
                            {
                                if files[n].lines[line].is_empty()
                                {
                                    placement = 0;
                                }
                                else
                                {
                                    placement = files[n].lines[line].len();
                                }
                            }
                            let s = start;
                            start = fix_top(start, placement, width);
                            if s != start
                            {
                                if line + 1 == top
                                {
                                    top -= 1;
                                }
                                clear(&files[n].lines, top, height, start, width);
                            }
                            else if line + 1 == top
                            {
                                top -= 1;
                                clear(&files[n].lines, top, height, start, width);
                            }
                        }
                        else if line + 1 == top
                        {
                            top -= 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    if mode == Search
                    {
                        ln = (line, placement);
                    }
                }
                '\x1E' | 'j' if c != 'j' || mode == Default =>
                {
                    //down
                    if line + 1 == files[n].lines.len()
                    {
                        if !files[n].lines[line].is_empty()
                        {
                            placement = files[n].lines[line].len();
                            files[n].cursor = placement;
                            let s = start;
                            start = fix_top(start, placement, width);
                            if s != start
                            {
                                clear(&files[n].lines, top, height, start, width);
                            }
                        }
                    }
                    else
                    {
                        line += 1;
                        if files[n].lines[line].is_empty()
                        {
                            placement = 0;
                        }
                        else if files[n].cursor != 0
                        {
                            if files[n].lines[line].len() > files[n].cursor
                            {
                                placement = files[n].cursor;
                            }
                            else if placement < files[n].cursor
                                || files[n].lines[line].len() < placement
                            {
                                placement = files[n].lines[line].len();
                            }
                        }
                        let s = start;
                        start = fix_top(start, placement, width);
                        if s != start
                        {
                            if line == height + top
                            {
                                top += 1;
                            }
                            clear(&files[n].lines, top, height, start, width);
                        }
                        else if line == height + top
                        {
                            top += 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    if mode == Search
                    {
                        ln = (line, placement);
                    }
                }
                '\x1A' => mode = Default, //esc
                '`' if mode == Default && n + 1 != files.len() =>
                {
                    //next file
                    files[n].placement = placement;
                    files[n].line = line;
                    files[n].start = start;
                    files[n].top = top;
                    n += 1;
                    print!("\x1b[H\x1b[J");
                    continue 'outer;
                }
                '~' if mode == Default && n != 0 =>
                {
                    //last file
                    files[n].placement = placement;
                    files[n].line = line;
                    files[n].start = start;
                    files[n].top = top;
                    n -= 1;
                    print!("\x1b[H\x1b[J");
                    continue 'outer;
                }
                '0' if mode == Default =>
                {
                    //start of line
                    placement = 0;
                    files[n].cursor = placement;
                    if start != 0
                    {
                        start = 0;
                        clear(&files[n].lines, top, height, start, width);
                    }
                    if mode == Search
                    {
                        ln = (line, placement);
                    }
                }
                '$' if mode == Default =>
                {
                    //end of line
                    placement = files[n].lines[line].len();
                    files[n].cursor = placement;
                    if placement > start + width
                    {
                        start = placement - width + 1;
                        clear(&files[n].lines, top, height, start, width);
                    }
                    if mode == Search
                    {
                        ln = (line, placement);
                    }
                }
                'w' if mode == Default =>
                {
                    //save
                    err = save_file(&mut files[n], &history_dir);
                    line = min(line, files[n].lines.len() - 1);
                    placement = min(placement, files[n].lines[line].len());
                    top = fix_top(top, line, height);
                    start = fix_top(start, placement, width);
                    if err.is_empty()
                    {
                        saved = true;
                    }
                }
                'y' if mode == Default =>
                {
                    //copy line
                    clip = files[n].lines[line].clone();
                }
                'd' if mode == Default =>
                {
                    //cut line
                    saved = false;
                    placement = 0;
                    files[n].cursor = 0;
                    start = 0;
                    if line + 1 == files[n].lines.len()
                    {
                        clip = files[n].lines.pop().unwrap();
                        files[n].lines.push(Vec::new());
                        print!("\x1b[G\x1b[K");
                    }
                    else
                    {
                        clip = files[n].lines.remove(line);
                        if top + height > files[n].lines.len()
                        {
                            print!("\x1b[G\x1b[J");
                        }
                        clear(&files[n].lines, top, height, start, width);
                    }
                    fix_history(&mut files[n].history);
                    files[n].history.list.insert(
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
                'p' if mode == Default =>
                {
                    //paste line
                    saved = false;
                    files[n].lines.insert(line, clip.clone());
                    placement = 0;
                    files[n].cursor = 0;
                    start = 0;
                    clear(&files[n].lines, top, height, start, width);
                    fix_history(&mut files[n].history);
                    files[n].history.list.insert(
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
                'r' if mode == Default =>
                {
                    //TODO
                    //replace
                }
                'R' if mode == Default =>
                {
                    //replace all
                    if let Ok(search) = get_word(&mut stdout, height)
                    {
                        let search = search.chars().collect::<Vec<char>>();
                        if let Ok(replace) = get_word(&mut stdout, height)
                        {
                            let replace = replace.chars().collect::<Vec<char>>();
                            for i in files[n].lines.iter_mut()
                            {
                                for j in 0..(i.len() - search.len() + 1)
                                {
                                    if i[j..j + search.len()] == search
                                    {
                                        i.splice(j..j + search.len(), replace.clone());
                                        //TODO support hist
                                    }
                                }
                            }
                            top = fix_top(top, line, height);
                            start = fix_top(start, placement, width);
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                }
                '/' if mode == Default =>
                {
                    //enable search
                    mode = Search;
                    orig = (line, placement);
                    word = Vec::new()
                }
                '\x14' | 'Q' if c != 'Q' || mode == Default => exit(), //ctrl+c
                'q' if mode == Default =>
                {
                    //quit
                    if saved
                    {
                        exit();
                    }
                    else
                    {
                        err = "unsaved changes, 'Q' to quit".to_string();
                    }
                }
                '\x05' if mode == Default || mode == Edit => mode = Insert,
                'i' if mode == Default => mode = Edit,
                'v' if mode == Default => mode = Digraph,
                's' if mode == Default =>
                {
                    if let Ok(file_path) = get_word(&mut stdout, height)
                    {
                        files[n].save_file_path = file_path;
                        err = save_file(&mut files[n], &history_dir);
                    };
                }
                'g' if mode == Default =>
                {
                    //goto line
                    if let Ok(number) = get_word(&mut stdout, height)
                    {
                        match number.parse::<usize>()
                        {
                            Ok(num) if num - 1 < files[n].lines.len() =>
                            {
                                line = num - 1;
                                if placement > files[n].lines[line].len()
                                {
                                    placement = files[n].lines[line].len();
                                }
                                let s = (top, start);
                                top = fix_top(top, line, height);
                                start = fix_top(start, placement, width);
                                if s != (top, start)
                                {
                                    clear(&files[n].lines, top, height, start, width);
                                }
                            }
                            Ok(_) => err = "file too short".to_string(),
                            Err(e) => err = e.to_string(),
                        }
                    }
                }
                'o' if mode == Default =>
                {
                    if let Ok(file_path) = get_word(&mut stdout, height)
                    {
                        let j = n;
                        let path = canonicalize(&file_path).unwrap();
                        for (index, file) in files.iter().enumerate()
                        {
                            if canonicalize(&file.save_file_path).unwrap() == path
                            {
                                n = index;
                            }
                        }
                        if n == j
                        {
                            files.push(open_file(&file_path, &history_dir));
                            n = files.len() - 1;
                        }
                        print!("\x1b[H\x1b[J");
                        continue 'outer;
                    };
                }
                '\0' =>
                {}
                _ if !c.is_ascii()
                    || c.is_ascii_graphic()
                    || c == ' '
                    || c == '\t'
                    || c == '\n' =>
                {
                    if mode == Edit || mode == Digraph || mode == Insert
                    {
                        saved = false;
                        if mode == Insert && files[n].lines[line].len() != placement
                        {
                            files[n].lines[line].remove(placement);
                        }
                        files[n].lines[line].insert(
                            placement,
                            if mode == Digraph
                            {
                                match c
                                {
                                    'a' => 'α',
                                    'A' => 'Α',
                                    'b' => 'β',
                                    'B' => 'Β',
                                    'c' => 'ξ',
                                    'C' => 'Ξ',
                                    'd' => 'Δ',
                                    'D' => 'δ',
                                    'e' => 'ε',
                                    'E' => 'Ε',
                                    'f' => 'φ',
                                    'F' => 'Φ',
                                    'g' => 'γ',
                                    'G' => 'Γ',
                                    'h' => 'η',
                                    'H' => 'Η',
                                    'i' => 'ι',
                                    'I' => 'Ι',
                                    'k' => 'κ',
                                    'K' => 'Κ',
                                    'l' => 'λ',
                                    'L' => 'Λ',
                                    'm' => 'μ',
                                    'M' => 'Μ',
                                    'n' => 'ν',
                                    'N' => 'Ν',
                                    'o' => 'ο',
                                    'O' => 'Ο',
                                    'p' => 'π',
                                    'P' => 'Π',
                                    'q' => 'θ',
                                    'Q' => 'Θ',
                                    'r' => 'ρ',
                                    'R' => 'Ρ',
                                    's' => 'σ',
                                    'S' => 'Σ',
                                    't' => 'τ',
                                    'T' => 'Τ',
                                    'u' => 'υ',
                                    'U' => 'Υ',
                                    'w' => 'ω',
                                    'W' => 'Ω',
                                    'y' => 'ψ',
                                    'Y' => 'Ψ',
                                    'x' => 'χ',
                                    'X' => 'Χ',
                                    'z' => 'ζ',
                                    'Z' => 'Ζ',
                                    _ => continue,
                                }
                            }
                            else
                            {
                                c
                            },
                        );
                        if placement + 1 == width + start
                        {
                            placement += 1;
                            files[n].cursor = placement;
                            start += 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                        else
                        {
                            clear_line(&files[n].lines, line, placement, width, start);
                            placement += 1;
                            files[n].cursor = placement;
                        }
                        fix_history(&mut files[n].history);
                        files[n].history.list.insert(
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
                    else if mode == Search
                    {
                        if c != '\n'
                        {
                            if start + width == placement + word.len() + 1
                                && files[n].lines[ln.0][ln.1 + word.len()] == c
                            {
                                start += 1;
                                clear(&files[n].lines, top, height, start, width);
                                stdout.flush().unwrap();
                            }
                            ln = orig;
                            word.push(c);
                        }
                        'inner: for (l, i) in files[n].lines.iter().enumerate()
                        {
                            if l >= ln.0 && word.len() <= i.len()
                            {
                                for j in
                                    if l == ln.0 { ln.1 + 1 } else { 0 }..(i.len() - word.len() + 1)
                                {
                                    if i[j..j + word.len()] == word
                                    {
                                        ln = (l, j);
                                        (line, placement) = ln;
                                        top = fix_top(top, line, height);
                                        start = fix_top(start, placement, width);
                                        if start + width == placement + 1
                                        {
                                            start += word.len()
                                        }
                                        files[n].cursor = placement;
                                        clear(&files[n].lines, top, height, start, width);
                                        break 'inner;
                                    }
                                }
                                ln = (0, 0);
                            }
                        }
                    }
                }
                'u' if mode == Default && files[n].history.list.len() != files[n].history.pos =>
                {
                    //undo
                    match (
                        files[n].history.list[files[n].history.pos].add,
                        files[n].history.list[files[n].history.pos].split,
                        files[n].history.list[files[n].history.pos].line.clone(),
                    )
                    {
                        (false, false, None) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = files[n].history.list[files[n].history.pos].pos.1;
                            if line == files[n].lines.len()
                            {
                                files[n].lines.push(Vec::new());
                            }
                            let char = files[n].history.list[files[n].history.pos].char;
                            files[n].lines[line].insert(placement, char);
                            placement += 1;
                        }
                        (true, false, None) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = files[n].history.list[files[n].history.pos].pos.1 - 1;
                            files[n].lines[line].remove(placement);
                        }
                        (false, true, None) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0 + 1;
                            placement = 0;
                            let pos = files[n].history.list[files[n].history.pos].pos.1;
                            let l = files[n].lines[line - 1].drain(pos..).collect();
                            files[n].lines.insert(line, l);
                        }
                        (true, true, None) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0 - 1;
                            placement = files[n].lines[line].len();
                            let l = files[n].lines.remove(line + 1);
                            files[n].lines[line].extend(l);
                        }
                        (false, false, Some(l)) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = 0;
                            if line == files[n].lines.len()
                            {
                                files[n].lines.push(l.clone());
                            }
                            else
                            {
                                files[n].lines.insert(line, l.clone());
                            }
                        }
                        (true, false, Some(_)) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = 0;
                            files[n].lines.remove(line);
                        }
                        (false, true, Some(l)) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = files[n].history.list[files[n].history.pos].pos.1;
                            let mut un = l.clone();
                            un.extend(files[n].lines[line].drain(placement..));
                            files[n].lines[line].extend(&un);
                            placement += l.len();
                        }
                        _ => unimplemented!(),
                    }
                    files[n].cursor = placement;
                    top = fix_top(top, line, height);
                    start = fix_top(start, placement, width);
                    clear(&files[n].lines, top, height, start, width);
                    files[n].history.pos += 1;
                }
                'U' if mode == Default && files[n].history.pos > 0 =>
                {
                    //redo
                    files[n].history.pos -= 1;
                    match (
                        files[n].history.list[files[n].history.pos].add,
                        files[n].history.list[files[n].history.pos].split,
                        files[n].history.list[files[n].history.pos].line.clone(),
                    )
                    {
                        (false, false, None) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = files[n].history.list[files[n].history.pos].pos.1;
                            files[n].lines[line].remove(placement);
                        }
                        (true, false, None) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = files[n].history.list[files[n].history.pos].pos.1 - 1;
                            let char = files[n].history.list[files[n].history.pos].char;
                            files[n].lines[line].insert(placement, char);
                            placement += 1;
                        }
                        (false, true, None) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = files[n].lines[line].len();
                            let l = files[n].lines.remove(line + 1);
                            files[n].lines[line].extend(l);
                        }
                        (true, true, None) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = 0;
                            if line == files[n].lines.len()
                            {
                                files[n].lines.push(Vec::new())
                            }
                            let pos = files[n].history.list[files[n].history.pos].pos.1;
                            let l = files[n].lines[line].drain(pos..).collect();
                            files[n].lines.insert(line, l);
                        }
                        (false, false, Some(_)) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = 0;
                            files[n].lines.remove(line);
                        }
                        (true, false, Some(l)) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = 0;
                            if line == files[n].lines.len()
                            {
                                files[n].lines.push(l.clone());
                            }
                            else
                            {
                                files[n].lines.insert(line, l.clone());
                            }
                        }
                        (false, true, Some(l)) =>
                        {
                            line = files[n].history.list[files[n].history.pos].pos.0;
                            placement = files[n].history.list[files[n].history.pos].pos.1;
                            files[n].lines[line].drain(placement..placement + l.len());
                        }
                        _ => unimplemented!(),
                    }
                    files[n].cursor = placement;
                    top = fix_top(top, line, height);
                    start = fix_top(start, placement, width);
                    clear(&files[n].lines, top, height, start, width);
                }
                _ =>
                {}
            }
            print_line_number(
                height,
                line,
                placement,
                top,
                start,
                if mode == Search
                {
                    word.iter().collect()
                }
                else
                {
                    err.clone()
                },
            );
            stdout.flush().unwrap();
        }
    }
}
#[derive(PartialEq)]
enum Mode
{
    Edit,
    Insert,
    Search,
    Digraph,
    Default,
}